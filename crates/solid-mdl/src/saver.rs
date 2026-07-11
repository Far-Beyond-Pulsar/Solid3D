//! `MdlSaver` — saves a `solid_rs::Scene` as a Quake MDL model file.

use std::io::Write;

use glam::{Vec2, Vec3};

use solid_rs::prelude::*;
use solid_rs::scene::Scene;
use solid_rs::{Result, SolidError};

use crate::constants;
use crate::parser::{self, MdlTriangle, MdlVertex};
use crate::MDL_FORMAT;

/// Saves a `Scene` as Quake MDL.
///
/// The saver collapses all scene meshes into a single MDL model,
/// computing appropriate scale/translate values from the combined
/// bounding box.  Vertex normals are quantised to the nearest entry
/// in the 162-element anorms table.  UV coordinates are converted
/// to the MDL integer format.  Textures are not saved (num_skins = 0).
pub struct MdlSaver;

impl Saver for MdlSaver {
    fn format_info(&self) -> &FormatInfo {
        &MDL_FORMAT
    }

    fn save(&self, scene: &Scene, writer: &mut dyn Write, _options: &SaveOptions) -> Result<()> {
        // Collect all vertices and indices from the scene
        let mut all_positions: Vec<Vec3> = Vec::new();
        let mut all_normals: Vec<Option<Vec3>> = Vec::new();
        let mut all_uvs: Vec<Option<Vec2>> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();
        let mut base_vertex: u32 = 0;

        for mesh in &scene.meshes {
            for v in &mesh.vertices {
                all_positions.push(v.position);
                all_normals.push(v.normal);
                all_uvs.push(v.uv());
            }
            for prim in &mesh.primitives {
                for idx in &prim.indices {
                    all_indices.push(base_vertex + idx);
                }
            }
            base_vertex += mesh.vertices.len() as u32;
        }

        let num_verts = all_positions.len();
        let num_tris = all_indices.len() / 3;

        if num_verts == 0 || num_tris == 0 {
            return Err(SolidError::format("mdl", "scene has no geometry to save"));
        }

        // Compute bounding box and scale/translate
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for p in &all_positions {
            min = min.min(*p);
            max = max.max(*p);
        }

        let range = max - min;
        let scale = [
            (range.x / 255.0).max(f32::EPSILON),
            (range.y / 255.0).max(f32::EPSILON),
            (range.z / 255.0).max(f32::EPSILON),
        ];
        let translate = [min.x, min.y, min.z];

        // Compress vertices and assign normal indices
        let mut mdl_verts: Vec<MdlVertex> = Vec::with_capacity(num_verts);
        for (i, pos) in all_positions.iter().enumerate() {
            let normal_index = match all_normals[i] {
                Some(n) => constants::find_closest_anorm(n),
                None => constants::find_closest_anorm(Vec3::Z),
            };
            let compressed = parser::compress_vertex(pos.to_array(), &scale, &translate);
            mdl_verts.push(MdlVertex {
                v: compressed.v,
                normal_index,
            });
        }

        // Convert UVs to MDL format
        let skinwidth = 64i32;
        let skinheight = 64i32;
        let mut texcoords = Vec::with_capacity(num_verts);
        for uv in &all_uvs {
            if let Some(u) = uv {
                let s = ((u.x * skinwidth as f32) - 0.5).round() as i32;
                let t = ((u.y * skinheight as f32) - 0.5).round() as i32;
                texcoords.push(parser::MdlTexCoord {
                    onseam: 0,
                    s: s.max(0).min(skinwidth - 1),
                    t: t.max(0).min(skinheight - 1),
                });
            } else {
                texcoords.push(parser::MdlTexCoord {
                    onseam: 0,
                    s: 0,
                    t: 0,
                });
            }
        }

        // Build triangles
        let mut triangles: Vec<MdlTriangle> = Vec::with_capacity(num_tris);
        for chunk in all_indices.chunks(3) {
            if chunk.len() < 3 {
                continue;
            }
            triangles.push(MdlTriangle {
                facesfront: 1,
                vertex: [chunk[0] as i32, chunk[1] as i32, chunk[2] as i32],
            });
        }
        let num_tris = triangles.len() as i32;

        // Build frame 0
        let zero_v = MdlVertex {
            v: [0, 0, 0],
            normal_index: 0,
        };
        let simple_frame = parser::MdlSimpleFrame {
            bboxmin: zero_v,
            bboxmax: zero_v,
            name: *b"frame_00        ",
            verts: mdl_verts,
        };

        // ── Write binary MDL ────────────────────────────────────────────────

        // Header
        write_u32_le(writer, parser::MDL_IDENT)?;
        write_i32_le(writer, parser::MDL_VERSION)?;
        write_f32_3(writer, scale)?;
        write_f32_3(writer, translate)?;
        write_f32_le(writer, range.length() * 0.5)?; // bounding radius
        write_f32_3(writer, [0.0f32; 3])?; // eyeposition
        write_i32_le(writer, 0)?; // num_skins (no textures saved)
        write_i32_le(writer, skinwidth)?;
        write_i32_le(writer, skinheight)?;
        write_i32_le(writer, num_verts as i32)?;
        write_i32_le(writer, num_tris)?;
        write_i32_le(writer, 1)?; // num_frames
        write_i32_le(writer, 0)?; // synctype
        write_i32_le(writer, 0)?; // flags
        write_f32_le(writer, 0.0)?; // size

        // No skins

        // Texture coordinates
        for tc in &texcoords {
            write_i32_le(writer, tc.onseam)?;
            write_i32_le(writer, tc.s)?;
            write_i32_le(writer, tc.t)?;
        }

        // Triangles
        for tri in &triangles {
            write_i32_le(writer, tri.facesfront)?;
            write_i32_le(writer, tri.vertex[0])?;
            write_i32_le(writer, tri.vertex[1])?;
            write_i32_le(writer, tri.vertex[2])?;
        }

        // Frame: simple frame
        write_i32_le(writer, 0)?; // type = simple
        write_vertex(writer, &simple_frame.bboxmin)?;
        write_vertex(writer, &simple_frame.bboxmax)?;
        writer.write_all(&simple_frame.name).map_err(SolidError::Io)?;
        for v in &simple_frame.verts {
            write_vertex(writer, v)?;
        }

        Ok(())
    }
}

// ── Binary write helpers ───────────────────────────────────────────────────────

fn write_u32_le(w: &mut dyn Write, v: u32) -> Result<()> {
    w.write_all(&v.to_le_bytes()).map_err(SolidError::Io)
}

fn write_i32_le(w: &mut dyn Write, v: i32) -> Result<()> {
    w.write_all(&v.to_le_bytes()).map_err(SolidError::Io)
}

fn write_f32_le(w: &mut dyn Write, v: f32) -> Result<()> {
    w.write_all(&v.to_le_bytes()).map_err(SolidError::Io)
}

fn write_f32_3(w: &mut dyn Write, v: [f32; 3]) -> Result<()> {
    write_f32_le(w, v[0])?;
    write_f32_le(w, v[1])?;
    write_f32_le(w, v[2])
}

fn write_vertex(w: &mut dyn Write, v: &MdlVertex) -> Result<()> {
    w.write_all(&[v.v[0], v.v[1], v.v[2], v.normal_index])
        .map_err(SolidError::Io)
}
