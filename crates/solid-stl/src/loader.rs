use std::collections::HashMap;
use std::io::Read;
use glam::Vec3;
use solid_rs::prelude::*;
use solid_rs::scene::Scene;

use crate::parser::{self, StlTriangle};
use crate::STL_FORMAT;

pub struct StlLoader;

fn build_mesh_data(triangles: &[StlTriangle]) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut map: HashMap<[u32; 3], u32> = HashMap::new();

    // Deduplicate by position; assign per-triangle color on first encounter.
    for tri in triangles {
        for &pos in tri.vertices.iter() {
            let key = [pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits()];
            let idx = if let Some(&existing) = map.get(&key) {
                existing
            } else {
                let new_idx = vertices.len() as u32;
                let mut v = Vertex::new(pos);
                if let Some(c) = tri.color {
                    v.colors[0] = Some(c);
                }
                vertices.push(v);
                map.insert(key, new_idx);
                new_idx
            };
            indices.push(idx);
        }
    }

    // Accumulate area-weighted face normals (unnormalized cross product = area weighting).
    let mut normal_accum: Vec<Vec3> = vec![Vec3::ZERO; vertices.len()];
    let mut normal_count: Vec<u32>  = vec![0; vertices.len()];

    for tri in triangles {
        let v0 = tri.vertices[0];
        let v1 = tri.vertices[1];
        let v2 = tri.vertices[2];
        let face_normal = (v1 - v0).cross(v2 - v0);

        for &pos in &tri.vertices {
            let key = [pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits()];
            if let Some(&idx) = map.get(&key) {
                normal_accum[idx as usize] += face_normal;
                normal_count[idx as usize] += 1;
            }
        }
    }

    for (i, v) in vertices.iter_mut().enumerate() {
        if normal_count[i] > 0 {
            let n = normal_accum[i].normalize_or_zero();
            if n != Vec3::ZERO {
                v.normal = Some(n);
            }
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

        let (vertices, indices) = build_mesh_data(&triangles);

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
