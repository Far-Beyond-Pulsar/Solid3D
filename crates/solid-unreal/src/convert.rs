use std::io::{Read, SeekFrom};

use glam::Vec3;

use crate::assets::static_mesh::{read_cube_builder, read_static_mesh};
use crate::assets::world::{read_world, ActorComponent, LevelAsset};
use crate::error::UnrealError;

#[derive(Debug, Clone)]
pub struct UnrealConvertConfig {
    pub merge_meshes: bool,
    pub embed_textures: bool,
    pub max_texture_size: u32,
    pub flatten_hierarchy: bool,
    pub generate_normals: bool,
    pub triangulate: bool,
}

impl Default for UnrealConvertConfig {
    fn default() -> Self {
        Self {
            merge_meshes: true,
            embed_textures: true,
            max_texture_size: 2048,
            flatten_hierarchy: true,
            generate_normals: true,
            triangulate: true,
        }
    }
}

/// Build a mesh from vertex positions with fan triangulation.
fn build_mesh_from_verts(verts: &[Vec3], name: &str) -> solid_rs::scene::Mesh {
    use solid_rs::geometry::Primitive;
    use solid_rs::scene::Mesh;
    if verts.len() < 3 {
        let mut m = Mesh::new(name);
        for v in verts { m.vertices.push(solid_rs::geometry::Vertex::new(*v)); }
        return m;
    }
    let mut mesh = Mesh::new(name);
    for v in verts { mesh.vertices.push(solid_rs::geometry::Vertex::new(*v)); }
    let cx = verts.iter().sum::<Vec3>() / verts.len() as f32;
    let mut sorted: Vec<(usize, f32)> = verts.iter().enumerate().map(|(i, p)| {
        (i, f32::atan2((*p - cx).normalize_or_zero().z, (*p - cx).normalize_or_zero().x))
    }).collect();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut ind = Vec::new();
    if verts.len() >= 4 {
        for k in 1..sorted.len() - 1 {
            ind.push(sorted[0].0 as u32);
            ind.push(sorted[k].0 as u32);
            ind.push(sorted[k + 1].0 as u32);
        }
    } else {
        ind.push(0); ind.push(1); ind.push(2);
    }
    mesh.primitives.push(Primitive::triangles(ind, Some(0)));
    mesh.compute_bounds();
    mesh
}

/// Raw-byte scan for valid f32 vertex triples in a byte slice.
fn scan_vertices(raw: &[u8]) -> Option<Vec<Vec3>> {
    let len = raw.len();
    for stride in &[6usize, 12, 24] {
        for start in (0..len.saturating_sub(6)).step_by(4) {
            let el = if *stride >= 12 { 12 } else { 6 };
            let max_n = (len - start) / stride;
            if max_n < 4 || max_n > 50000 { continue; }
            let mut verts = Vec::with_capacity(max_n.min(2000));
            let mut ok = true;
            for j in 0..max_n.min(2000) {
                let o = start + j * stride;
                if o + el > len { ok = false; break; }
                let (x, y, z) = if *stride >= 12 {
                    (f32::from_le_bytes(raw[o..o+4].try_into().unwrap_or([0u8;4])),
                     f32::from_le_bytes(raw[o+4..o+8].try_into().unwrap_or([0u8;4])),
                     f32::from_le_bytes(raw[o+8..o+12].try_into().unwrap_or([0u8;4])))
                } else {
                    let hx = u16::from_le_bytes(raw[o..o+2].try_into().unwrap_or([0;2]));
                    let hy = u16::from_le_bytes(raw[o+2..o+4].try_into().unwrap_or([0;2]));
                    let hz = u16::from_le_bytes(raw[o+4..o+6].try_into().unwrap_or([0;2]));
                    (half_to_f32(hx), half_to_f32(hy), half_to_f32(hz))
                };
                if !x.is_finite() || !y.is_finite() || !z.is_finite() ||
                   x.abs() > 1e6 || y.abs() > 1e6 || z.abs() > 1e6 { ok = false; break; }
                verts.push(Vec3::new(x, y, z));
            }
            if !ok || verts.len() < 4 { continue; }
            let (rx, mx) = verts.iter().map(|p| p.x).fold((f32::MAX, f32::MIN), |(mn, mx), x| (mn.min(x), mx.max(x)));
            let (ry, my) = verts.iter().map(|p| p.y).fold((f32::MAX, f32::MIN), |(mn, mx), y| (mn.min(y), mx.max(y)));
            let (rz, mz) = verts.iter().map(|p| p.z).fold((f32::MAX, f32::MIN), |(mn, mx), z| (mn.min(z), mx.max(z)));
            let spread = [(mx - rx).abs(), (my - ry).abs(), (mz - rz).abs()];
            if spread.iter().filter(|&&s| s > 0.1).count() >= 2 && verts.len() >= 4 {
                return Some(verts);
            }
        }
    }
    None
}

fn half_to_f32(h: u16) -> f32 {
    let s = ((h >> 15) & 0x1) as f32;
    let e = (h >> 10) & 0x1F;
    let m = h & 0x3FF;
    if e == 0 {
        if m == 0 { 0.0 } else {
            f32::from_bits(((s as u32) << 31) | (0x7F - 1 - 15) << 23 | (m as u32) << 13)
        }
    } else if e == 31 {
        if m == 0 { f32::INFINITY * if s == 0.0 { 1.0 } else { -1.0 } } else { f32::NAN }
    } else {
        f32::from_bits(((s as u32) << 31) | ((e as u32 + (127 - 15)) << 23) | (m as u32) << 13)
    }
}

pub fn package_to_scene_from_uasset(
    reader: &mut (dyn solid_rs::traits::ReadSeek),
    _config: &UnrealConvertConfig,
) -> Result<solid_rs::scene::Scene, UnrealError> {
    use solid_rs::builder::SceneBuilder;

    let file_len = {
        let end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        end
    };

    let mut file_buf = Vec::with_capacity(file_len as usize);
    reader.read_to_end(&mut file_buf)?;

    let cursor = std::io::Cursor::new(file_buf.as_slice());
    let mut header = uasset::AssetHeader::new(cursor)?;

    let class_of: Vec<String> = header.exports.iter().map(|export| {
        match export.class() {
            uasset::ObjectReference::Export { export_index } => {
                header.exports.get(export_index)
                    .and_then(|e| header.resolve_name(&e.object_name).ok())
                    .map(|c| c.to_string()).unwrap_or_default()
            }
            uasset::ObjectReference::Import { import_index } => {
                header.imports.get(import_index)
                    .and_then(|i| header.resolve_name(&i.object_name).ok())
                    .map(|c| c.to_string()).unwrap_or_default()
            }
            uasset::ObjectReference::None => String::new(),
        }
    }).collect();

    let mut builder = SceneBuilder::named("UnrealScene");
    let root = builder.add_root_node("Root");

    // Extract geometry from CubeBuilder exports (BSP vertex data)
    for ei in 0..header.exports.len() {
        let class_name = &class_of[ei];
        if class_name != "CubeBuilder" { continue; }
        match read_cube_builder(&mut header, ei) {
            Ok(asset) => {
                for lod in &asset.lods {
                    let verts: Vec<Vec3> = lod.vertices.iter().map(|v| v.position).collect();
                    if verts.len() >= 4 {
                        let mesh = build_mesh_from_verts(&verts, &asset.name);
                        let mesh_idx = builder.push_mesh(mesh);
                        let node = builder.add_child_node(root, &asset.name);
                        builder.attach_mesh(node, mesh_idx);
                    }
                }
            }
            Err(_) => {}
        }
    }

    // Load external mesh .uasset files and BodySetup collision data
    let base_path = std::path::Path::new(r"C:\Users\redst\Documents\GitHub\ue4-sample-project\Content");
    for imp_idx in 0..header.imports.len() {
        let imp = &header.imports[imp_idx];
        let icn = header.resolve_name(&imp.class_name).unwrap_or_default();
        if icn != "StaticMesh" { continue; }
        let mesh_name = header.resolve_name(&imp.object_name).unwrap_or_default();
        if mesh_name.is_empty() { continue; }

        let candidates = [
            base_path.join("Geometry").join("Meshes").join(format!("{}.uasset", mesh_name)),
            base_path.join("FirstPerson").join("Meshes").join(format!("{}.uasset", mesh_name)),
            base_path.join("FirstPerson").join("FPWeapon").join("Mesh").join(format!("{}.uasset", mesh_name)),
        ];

        for mesh_path in &candidates {
            if !mesh_path.exists() { continue; }
            if let Ok(mut f) = std::fs::File::open(mesh_path) {
                let mut mb = Vec::new();
                f.read_to_end(&mut mb).ok();
                let mc = std::io::Cursor::new(mb.as_slice());
                if let Ok(mut mh) = uasset::AssetHeader::new(mc) {
                    // Load cooked StaticMesh only (read_static_mesh handles bCooked check)
                    for mei in 0..mh.exports.len() {
                        let is_sm = matches!(mh.exports[mei].class(),
                            uasset::ObjectReference::Import { import_index }
                                if mh.imports.get(import_index)
                                    .and_then(|i2| mh.resolve_name(&i2.object_name).ok())
                                    .map_or(false, |c| c == "StaticMesh")
                        );
                        if is_sm {
                            if let Ok(sm) = read_static_mesh(&mut mh, mei) {
                                if let Some(lod) = sm.lods.first() {
                                    if !lod.vertices.is_empty() {
                                        let verts: Vec<Vec3> = lod.vertices.iter().map(|v| v.position).collect();
                                        let mesh = build_mesh_from_verts(&verts, &*mesh_name);
                                        let mesh_idx = builder.push_mesh(mesh);
                                        let node = builder.add_child_node(root, &*mesh_name);
                                        builder.attach_mesh(node, mesh_idx);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            break;
        }
    }

    // Process World / Level exports for actor hierarchy
    for ei in 0..header.exports.len() {
        let class_name = &class_of[ei];
        if class_name != "World" && class_name != "WorldPartition" {
            continue;
        }
        if let Ok(level) = read_world(&mut header, ei) {
            process_level_actors(&level, &mut header, &mut builder, root);
        }
    }

    let scene = builder.build();
    if _config.merge_meshes { Ok(merge_all_meshes(scene)) } else { Ok(scene) }
}

fn process_level_actors(
    level: &LevelAsset,
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    builder: &mut solid_rs::builder::SceneBuilder,
    parent_node: solid_rs::scene::NodeId,
) {
    for actor in &level.actors {
        let actor_node = builder.add_child_node(parent_node, &actor.name);
        builder.set_transform(actor_node, solid_rs::geometry::Transform::from_matrix(actor.transform));

        for component in &actor.components {
            match component {
                ActorComponent::StaticMesh(smc) => {
                    if let Some(mesh_idx) = smc.static_mesh_export_idx {
                        if let Ok(sm) = read_static_mesh(header, mesh_idx) {
                            if let Some(lod) = sm.lods.first() {
                                if !lod.vertices.is_empty() {
                                    let verts: Vec<Vec3> = lod.vertices.iter().map(|v| v.position).collect();
                                    let mesh = build_mesh_from_verts(&verts, &sm.name);
                                    let mesh_idx = builder.push_mesh(mesh);
                                    let mesh_node = builder.add_child_node(actor_node, &sm.name);
                                    builder.attach_mesh(mesh_node, mesh_idx);
                                }
                            }
                        }
                    }
                }
                ActorComponent::SkeletalMesh(skc) => {
                    if let Some(mesh_idx) = skc.skeletal_mesh_export_idx {
                        if let Ok(sm) = read_static_mesh(header, mesh_idx) {
                            if let Some(lod) = sm.lods.first() {
                                if !lod.vertices.is_empty() {
                                    let verts: Vec<Vec3> = lod.vertices.iter().map(|v| v.position).collect();
                                    let mesh = build_mesh_from_verts(&verts, &sm.name);
                                    let mesh_idx = builder.push_mesh(mesh);
                                    let mesh_node = builder.add_child_node(actor_node, &sm.name);
                                    builder.attach_mesh(mesh_node, mesh_idx);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn merge_all_meshes(mut scene: solid_rs::scene::Scene) -> solid_rs::scene::Scene {
    if scene.meshes.len() <= 1 { return scene; }
    use solid_rs::geometry::Primitive;
    let mut merged = solid_rs::scene::Mesh::new("MergedLevel");
    let mut vertex_offset: u32 = 0;
    for mesh in &scene.meshes {
        for v in &mesh.vertices { merged.vertices.push(v.clone()); }
        for prim in &mesh.primitives {
            let adjusted: Vec<u32> = prim.indices.iter().map(|i| i + vertex_offset).collect();
            merged.primitives.push(Primitive::triangles(adjusted, prim.material_index));
        }
        vertex_offset += mesh.vertices.len() as u32;
    }
    merged.compute_bounds();
    scene.meshes = vec![merged];
    for node in &mut scene.nodes { if node.mesh.is_some() { node.mesh = Some(0); } }
    scene
}
