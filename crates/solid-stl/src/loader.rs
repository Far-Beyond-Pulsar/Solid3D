use std::collections::HashMap;
use std::io::Read;
use solid_rs::prelude::*;
use solid_rs::scene::Scene;

use crate::parser::{self, StlTriangle};
use crate::STL_FORMAT;

pub struct StlLoader;

fn deduplicate(triangles: &[StlTriangle]) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut map: HashMap<[u32; 3], u32> = HashMap::new();

    for tri in triangles {
        for &pos in tri.vertices.iter() {
            let key = [pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits()];
            let idx = if let Some(&existing) = map.get(&key) {
                existing
            } else {
                let new_idx = vertices.len() as u32;
                let v = Vertex::new(pos).with_normal(tri.normal);
                vertices.push(v);
                map.insert(key, new_idx);
                new_idx
            };
            indices.push(idx);
        }
    }
    (vertices, indices)
}

impl Loader for StlLoader {
    fn format_info(&self) -> &FormatInfo {
        &STL_FORMAT
    }

    fn load(&self, reader: &mut dyn ReadSeek, _options: &LoadOptions) -> solid_rs::Result<Scene> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(SolidError::Io)?;

        let (name, triangles) = if parser::detect_binary(&data) {
            parser::parse_binary(&data)?
        } else {
            match parser::parse_ascii(&data) {
                Ok(result) => result,
                Err(_) => {
                    // Some binary STLs start with "solid" — try binary as fallback
                    parser::parse_binary(&data)?
                }
            }
        };

        let (vertices, indices) = deduplicate(&triangles);

        let mesh_name = if name.is_empty() { "STL Model".to_string() } else { name.clone() };
        let mut mesh = Mesh::new(mesh_name.clone());
        mesh.vertices = vertices;
        mesh.primitives.push(Primitive::triangles(indices, None));

        let scene_name = if name.is_empty() { "STL Scene".to_string() } else { name };
        let mut builder = SceneBuilder::named(scene_name);
        let mesh_idx = builder.push_mesh(mesh);
        let root = builder.add_root_node(mesh_name);
        builder.attach_mesh(root, mesh_idx);

        Ok(builder.build())
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 256];
        let n = reader.read(&mut buf).unwrap_or(0);
        let slice = &buf[..n];
        // ASCII STL starts with "solid"
        if slice.starts_with(b"solid") {
            return 0.7;
        }
        // Binary: just say it might be if >= 84 bytes worth are peeked
        if n >= 84 {
            return 0.5;
        }
        0.0
    }
}
