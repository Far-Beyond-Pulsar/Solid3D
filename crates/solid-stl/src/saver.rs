use std::io::Write;
use solid_rs::prelude::*;
use solid_rs::scene::Scene;

use crate::STL_FORMAT;

pub struct StlSaver;

impl StlSaver {
    /// Write ASCII STL to `writer`.
    pub fn save_ascii(&self, scene: &Scene, writer: &mut dyn Write, options: &SaveOptions) -> solid_rs::Result<()> {
        let gen = options.generator.as_deref().unwrap_or("solid-stl");
        for mesh in &scene.meshes {
            let solid_name = if mesh.name.is_empty() { gen.to_string() } else { mesh.name.clone() };
            writeln!(writer, "solid {solid_name}").map_err(SolidError::Io)?;

            for prim in &mesh.primitives {
                for tri_indices in prim.indices.chunks(3) {
                    if tri_indices.len() < 3 {
                        continue;
                    }
                    let v0 = &mesh.vertices[tri_indices[0] as usize];
                    let v1 = &mesh.vertices[tri_indices[1] as usize];
                    let v2 = &mesh.vertices[tri_indices[2] as usize];

                    let normal = face_normal(v0.position, v1.position, v2.position);
                    writeln!(writer, "  facet normal {:.6} {:.6} {:.6}", normal.x, normal.y, normal.z).map_err(SolidError::Io)?;
                    writeln!(writer, "    outer loop").map_err(SolidError::Io)?;
                    write_vertex(writer, v0.position)?;
                    write_vertex(writer, v1.position)?;
                    write_vertex(writer, v2.position)?;
                    writeln!(writer, "    endloop").map_err(SolidError::Io)?;
                    writeln!(writer, "  endfacet").map_err(SolidError::Io)?;
                }
            }

            writeln!(writer, "endsolid {solid_name}").map_err(SolidError::Io)?;
        }
        Ok(())
    }
}

fn write_vertex(writer: &mut dyn Write, pos: glam::Vec3) -> solid_rs::Result<()> {
    writeln!(writer, "      vertex {:.6} {:.6} {:.6}", pos.x, pos.y, pos.z).map_err(SolidError::Io)
}

fn face_normal(v0: glam::Vec3, v1: glam::Vec3, v2: glam::Vec3) -> glam::Vec3 {
    let n = (v1 - v0).cross(v2 - v0);
    let len = n.length();
    if len > 1e-10 { n / len } else { glam::Vec3::Z }
}

fn count_triangles(scene: &Scene) -> u32 {
    let mut count = 0u32;
    for mesh in &scene.meshes {
        for prim in &mesh.primitives {
            count += (prim.indices.len() / 3) as u32;
        }
    }
    count
}

impl Saver for StlSaver {
    fn format_info(&self) -> &FormatInfo {
        &STL_FORMAT
    }

    /// Write binary STL (default — smaller and faster than ASCII).
    fn save(&self, scene: &Scene, writer: &mut dyn Write, options: &SaveOptions) -> solid_rs::Result<()> {
        // Build 80-byte header from the first mesh name (or generator).
        let mesh_name = scene.meshes.first().map(|m| m.name.as_str()).unwrap_or("");
        let header_str = if mesh_name.is_empty() {
            options.generator.as_deref().unwrap_or("solid-stl").to_string()
        } else {
            mesh_name.to_string()
        };

        let mut header = [0u8; 80];
        let bytes = header_str.as_bytes();
        let copy_len = bytes.len().min(80);
        header[..copy_len].copy_from_slice(&bytes[..copy_len]);
        writer.write_all(&header).map_err(SolidError::Io)?;

        // Write triangle count.
        let tri_count = count_triangles(scene);
        writer.write_all(&tri_count.to_le_bytes()).map_err(SolidError::Io)?;

        for mesh in &scene.meshes {
            for prim in &mesh.primitives {
                for tri_indices in prim.indices.chunks(3) {
                    if tri_indices.len() < 3 {
                        continue;
                    }
                    let v0 = &mesh.vertices[tri_indices[0] as usize];
                    let v1 = &mesh.vertices[tri_indices[1] as usize];
                    let v2 = &mesh.vertices[tri_indices[2] as usize];

                    let normal = face_normal(v0.position, v1.position, v2.position);

                    write_f32_3(writer, normal)?;
                    write_f32_3(writer, v0.position)?;
                    write_f32_3(writer, v1.position)?;
                    write_f32_3(writer, v2.position)?;
                    // Attribute byte count = 0
                    writer.write_all(&0u16.to_le_bytes()).map_err(SolidError::Io)?;
                }
            }
        }
        Ok(())
    }
}

fn write_f32_3(writer: &mut dyn Write, v: glam::Vec3) -> solid_rs::Result<()> {
    writer.write_all(&v.x.to_le_bytes()).map_err(SolidError::Io)?;
    writer.write_all(&v.y.to_le_bytes()).map_err(SolidError::Io)?;
    writer.write_all(&v.z.to_le_bytes()).map_err(SolidError::Io)
}
