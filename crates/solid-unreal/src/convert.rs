use std::io::SeekFrom;

use glam::{Vec3, Vec4};

use crate::assets::static_mesh::UnrealVertex;
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

/// Convert a .umap to Scene, using uasset crate for correct package parsing.
pub fn package_to_scene_from_uasset(
    reader: &mut (dyn solid_rs::traits::ReadSeek),
    config: &UnrealConvertConfig,
) -> Result<solid_rs::scene::Scene, UnrealError> {
    use solid_rs::builder::SceneBuilder;
    use solid_rs::geometry::Primitive;
    use solid_rs::scene::Mesh;

    let file_len = {
        let end = reader.seek(SeekFrom::End(0)).map_err(|e| UnrealError::Io(e))?;
        reader.seek(SeekFrom::Start(0)).map_err(|e| UnrealError::Io(e))?;
        end
    };

    let mut file_buf = Vec::with_capacity(file_len as usize);
    reader.read_to_end(&mut file_buf).map_err(|e| UnrealError::Io(e))?;
    let cursor = std::io::Cursor::new(&file_buf);

    let uasset_header = uasset::AssetHeader::new(cursor)
        .map_err(|e| UnrealError::Parse { context: "uasset", detail: format!("{e}") })?;

    let mut builder = SceneBuilder::named("UnrealScene");
    let root = builder.add_root_node("Root");

    for export in &uasset_header.exports {
        let class_ref = export.class();
        let class_name = match class_ref {
            uasset::ObjectReference::Export { export_index } => {
                uasset_header.exports.get(export_index)
                    .and_then(|e| uasset_header.resolve_name(&e.object_name).ok())
                    .map(|c| c.to_string()).unwrap_or_default()
            }
            uasset::ObjectReference::Import { import_index } => {
                uasset_header.imports.get(import_index)
                    .and_then(|i| uasset_header.resolve_name(&i.object_name).ok())
                    .map(|c| c.to_string()).unwrap_or_default()
            }
            uasset::ObjectReference::None => String::new(),
        };

        if class_name.is_empty() || export.serial_size <= 0 { continue; }
        if export.serial_offset < 0 || (export.serial_offset as u64 + export.serial_size as u64) > file_len {
            continue;
        }

        let off = export.serial_offset as usize;
        let sz = export.serial_size as usize;
        let data = &file_buf[off..off + sz];

        let name = uasset_header.resolve_name(&export.object_name)
            .unwrap_or_default().to_string();

        // Try to extract vertex data from any export class
        let mut positions = Vec::new();
        for start_byte in (0..sz.saturating_sub(12)).step_by(4) {
            let max_count = (sz - start_byte) / 12;
            if max_count < 3 || max_count > 500 { continue; }
            let mut candidate = Vec::with_capacity(max_count);
            let mut valid = true;
            for i in 0..max_count {
                let o = start_byte + i * 12;
                if o + 12 > sz { break; }
                let x = f32::from_le_bytes(data[o..o+4].try_into().unwrap());
                let y = f32::from_le_bytes(data[o+4..o+8].try_into().unwrap());
                let z = f32::from_le_bytes(data[o+8..o+12].try_into().unwrap());
                if !x.is_finite() || !y.is_finite() || !z.is_finite() ||
                   x.abs() > 1e6 || y.abs() > 1e6 || z.abs() > 1e6 { valid = false; break; }
                candidate.push(Vec3::new(x, y, z));
            }
            if valid && candidate.len() >= 3 {
                let rx = candidate.iter().map(|p| p.x).fold(f32::MAX, f32::min);
                let mx = candidate.iter().map(|p| p.x).fold(f32::MIN, f32::max);
                let ry = candidate.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                let my = candidate.iter().map(|p| p.y).fold(f32::MIN, f32::max);
                let rz = candidate.iter().map(|p| p.z).fold(f32::MAX, f32::min);
                let mz = candidate.iter().map(|p| p.z).fold(f32::MIN, f32::max);
                let rxr = (mx - rx).abs();
                let ryr = (my - ry).abs();
                let rzr = (mz - rz).abs();
                // Require at least 2 axes with spread > 1.0 (rejects 1D byte-garbage)
                let valid_axes = [rxr > 1.0, ryr > 1.0, rzr > 1.0].iter().filter(|&&x| x).count();
                if valid_axes >= 2 && candidate.len() >= 4 {
                    positions = candidate;
                    break;
                }
            }
        }

        if positions.len() < 3 { continue; }

        // Build mesh: the data is polygon vertices; fan triangulate from centroid
        let centroid = positions.iter().sum::<Vec3>() / positions.len() as f32;
        let ue_verts: Vec<UnrealVertex> = positions.iter().map(|p| {
            UnrealVertex { position: *p, ..Default::default() }
        }).collect();

        // Sort vertices around centroid for proper convex polygon fan triangulation
        let mut sorted: Vec<(usize, f32)> = positions.iter().enumerate().map(|(i, p)| {
            let dir = (*p - centroid).normalize_or_zero();
            (i, f32::atan2(dir.z, dir.x))
        }).collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Fan triangulation: hub = sorted[0], triangles = (hub, sorted[i], sorted[i+1])
        let hub = sorted[0].0 as u32;
        let mut indices = Vec::new();
        for i in 1..sorted.len() - 1 {
            indices.push(hub);
            indices.push(sorted[i].0 as u32);
            indices.push(sorted[i + 1].0 as u32);
        }

        let mut mesh = Mesh::new(&name);
        for v in &ue_verts {
            let mut sv = solid_rs::geometry::Vertex::new(v.position);
            sv.normal = Some(v.normal);
            sv.tangent = Some(Vec4::new(v.tangent.x, v.tangent.y, v.tangent.z, v.tangent.w));
            sv.uvs[0] = Some(v.uv[0]);
            sv.colors[0] = Some(Vec4::new(
                v.color[0] as f32 / 255.0, v.color[1] as f32 / 255.0,
                v.color[2] as f32 / 255.0, v.color[3] as f32 / 255.0,
            ));
            mesh.vertices.push(sv);
        }
        mesh.primitives.push(Primitive::triangles(indices, Some(0)));
        mesh.compute_bounds();

        let mesh_idx = builder.push_mesh(mesh);
        builder.attach_mesh(root, mesh_idx);
    }

    let scene = builder.build();
    if config.merge_meshes {
        Ok(merge_all_meshes(scene))
    } else {
        Ok(scene)
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
    for node in &mut scene.nodes {
        if node.mesh.is_some() { node.mesh = Some(0); }
    }
    scene
}
