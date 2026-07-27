use std::io::SeekFrom;

use glam::Vec3;

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

pub fn package_to_scene_from_uasset(
    reader: &mut (dyn solid_rs::traits::ReadSeek),
    _config: &UnrealConvertConfig,
) -> Result<solid_rs::scene::Scene, UnrealError> {
    use solid_rs::builder::SceneBuilder;
    use solid_rs::geometry::Primitive;
    use solid_rs::scene::Mesh;

    let file_len = {
        let end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        end
    };

    let mut file_buf = Vec::with_capacity(file_len as usize);
    reader.read_to_end(&mut file_buf)?;

    let cursor = std::io::Cursor::new(&file_buf);
    let header = uasset::AssetHeader::new(cursor)?;

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

    for (ei, export) in header.exports.iter().enumerate() {
        let class_name = &class_of[ei];
        if !["BrushComponent", "StaticMeshComponent", "CapsuleComponent", "Polys", "Model"].contains(&class_name.as_str()) {
            continue;
        }
        if export.serial_size < 16 || export.serial_offset < 0 { continue; }
        let off = export.serial_offset as usize;
        let sz = export.serial_size as usize;
        if off + sz > file_buf.len() { continue; }

        let name = header.resolve_name(&export.object_name)
            .unwrap_or_default().to_string();
        let raw = &file_buf[off..off + sz];

        let mut best: Vec<Vec3> = Vec::new();
        for start in (0..sz - 12).step_by(4) {
            let max_n = (sz - start) / 12;
            if max_n < 3 || max_n > 500 { continue; }
            let mut verts = Vec::with_capacity(max_n);
            let mut ok = true;
            for i in 0..max_n {
                let o = start + i * 12;
                let x = f32::from_le_bytes(raw[o..o+4].try_into().unwrap_or([0u8;4]));
                let y = f32::from_le_bytes(raw[o+4..o+8].try_into().unwrap_or([0u8;4]));
                let z = f32::from_le_bytes(raw[o+8..o+12].try_into().unwrap_or([0u8;4]));
                if !x.is_finite() || !y.is_finite() || !z.is_finite() ||
                   x.abs() > 1e6 || y.abs() > 1e6 || z.abs() > 1e6 { ok = false; break; }
                verts.push(Vec3::new(x, y, z));
            }
            if !ok || verts.len() < 4 { continue; }
            let rx = verts.iter().map(|p| p.x).fold(f32::MAX, f32::min);
            let mx = verts.iter().map(|p| p.x).fold(f32::MIN, f32::max);
            let ry = verts.iter().map(|p| p.y).fold(f32::MAX, f32::min);
            let my = verts.iter().map(|p| p.y).fold(f32::MIN, f32::max);
            let rz = verts.iter().map(|p| p.z).fold(f32::MAX, f32::min);
            let mz = verts.iter().map(|p| p.z).fold(f32::MIN, f32::max);
            let axes_ok = [(mx - rx).abs() > 1.0, (my - ry).abs() > 1.0, (mz - rz).abs() > 1.0];
            if axes_ok.iter().filter(|&&x| x).count() >= 2 {
                best = verts;
                break;
            }
        }
        if best.len() < 4 { continue; }

        let centroid = best.iter().sum::<Vec3>() / best.len() as f32;
        let mut sorted: Vec<(usize, f32)> = best.iter().enumerate().map(|(i, p)| {
            let dir = (*p - centroid).normalize_or_zero();
            (i, f32::atan2(dir.z, dir.x))
        }).collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let hub = sorted[0].0 as u32;
        let mut indices = Vec::new();
        for i in 1..sorted.len() - 1 {
            indices.push(hub);
            indices.push(sorted[i].0 as u32);
            indices.push(sorted[i + 1].0 as u32);
        }

        let mut mesh = Mesh::new(&name);
        for v in &best { mesh.vertices.push(solid_rs::geometry::Vertex::new(*v)); }
        mesh.primitives.push(Primitive::triangles(indices, Some(0)));
        mesh.compute_bounds();
        let mesh_idx = builder.push_mesh(mesh);
        let node = builder.add_child_node(root, &name);
        builder.attach_mesh(node, mesh_idx);
    }

    let scene = builder.build();
    if _config.merge_meshes { Ok(merge_all_meshes(scene)) } else { Ok(scene) }
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
