//! Converts parsed [`MdlData`] into a `solid_rs::Scene`.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use solid_rs::builder::SceneBuilder;
use solid_rs::geometry::{Primitive, Vertex};
use solid_rs::scene::{Image, ImageSource, Material, Mesh, Scene, Texture, TextureRef};
use solid_rs::{Result, SolidError};

use crate::constants;
use crate::parser::{self, MdlData, MdlFrame};

/// Convert parsed MDL data into a `solid_rs::Scene`.
///
/// The loader takes the first frame of the model and constructs a single
/// mesh with positions, normals, UVs, and optionally an embedded texture
/// image from the first skin.
pub fn mdl_to_scene(data: &MdlData) -> Result<Scene> {
    let header = &data.header;
    let scale = &header.scale;
    let translate = &header.translate;
    let skinwidth = header.skinwidth.max(0) as f32;
    let skinheight = header.skinheight.max(0) as f32;

    let num_verts = header.num_verts.max(0) as usize;
    let num_tris = header.num_tris.max(0) as usize;

    if num_verts == 0 || num_tris == 0 {
        let b = SceneBuilder::named("MDL Model (empty)");
        return Ok(b.build());
    }

    // Get frame 0 vertices
    let frame_verts = match data.frames.first() {
        Some(MdlFrame::Simple(sf)) => &sf.verts,
        Some(MdlFrame::Group { frames, .. }) => {
            if let Some(sf) = frames.first() {
                &sf.verts
            } else {
                return Err(SolidError::parse("MDL frame group is empty"));
            }
        }
        None => {
            return Err(SolidError::parse("MDL has no frames"));
        }
    };

    if frame_verts.len() != num_verts {
                return Err(SolidError::parse(format!(
            "MDL vertex count mismatch: header says {}, frame has {}",
            num_verts,
            frame_verts.len()
        )));
    }

    // ── Material & texture ─────────────────────────────────────────────────

    let mut b = SceneBuilder::named("MDL Model");
    let mut has_texture = false;

    if let Some(skin) = data.skins.first() {
        let pixel_data = match skin {
            parser::MdlSkin::Single { data } => data.as_slice(),
            parser::MdlSkin::Group { data, .. } => {
                data.first().map(|v| v.as_slice()).unwrap_or(&[])
            }
        };

        if !pixel_data.is_empty() && skinwidth > 0.0 && skinheight > 0.0 {
            let w = header.skinwidth.max(0) as usize;
            let h = header.skinheight.max(0) as usize;
            let count = pixel_data.len().min(w * h);

            // Convert indexed 8-bit to RGBA 32-bit using the Quake palette
            let mut rgba = Vec::with_capacity(count * 4);
            for i in 0..count {
                let idx = pixel_data[i] as usize;
                let c = &constants::COLORMAP[idx.min(255)];
                if idx == 255 {
                    rgba.push(c[0]);
                    rgba.push(c[1]);
                    rgba.push(c[2]);
                    rgba.push(0);
                } else {
                    rgba.push(c[0]);
                    rgba.push(c[1]);
                    rgba.push(c[2]);
                    rgba.push(255);
                }
            }

            let png_data = encode_rgba_to_png(&rgba, w, h);
            let img = Image {
                name: "MDL Skin".to_string(),
                source: ImageSource::Embedded {
                    mime_type: "image/png".to_string(),
                    data: png_data,
                },
                extensions: Default::default(),
            };
            let img_idx = b.push_image(img);
            let tex = Texture::new("MDL Skin", img_idx);
            let tex_idx = b.push_texture(tex);

            let mut mat = Material::new("MDL Default");
            mat.base_color_texture = Some(TextureRef::new(tex_idx));
            mat.metallic_factor = 0.0;
            mat.roughness_factor = 1.0;
            b.push_material(mat);
            has_texture = true;
        }
    }

    if !has_texture {
        b.push_material(Material::new("MDL Default"));
    }

    // ── Build vertices with UV splitting ────────────────────────────────────
    //
    // MDL vertices can have different UVs depending on the triangle's
    // facesfront flag and the vertex's onseam flag.  We split vertices
    // where the effective UV differs between triangles.

    let mut solid_verts: Vec<Vertex> = Vec::new();
    let mut vert_map: HashMap<(usize, i32, i32), u32> = HashMap::new();
    let mut indices: Vec<u32> = Vec::with_capacity(num_tris * 3);

    for tri in &data.triangles {
        for &vi in &tri.vertex {
            let idx = vi as usize;
            if idx >= num_verts {
        return Err(SolidError::parse(format!(
                    "MDL vertex index {} out of range (num_verts = {})",
                    idx, num_verts
                )));
            }

            let tc = &data.texcoords[idx];
            let (eff_s, eff_t) = if tri.facesfront == 0 && tc.onseam != 0 {
                (tc.s + header.skinwidth / 2, tc.t)
            } else {
                (tc.s, tc.t)
            };

            let key = (idx, eff_s, eff_t);
            let vert_idx = *vert_map.entry(key).or_insert_with(|| {
                let pos = parser::decompress_vertex(&frame_verts[idx], scale, translate);
                let mut v = Vertex::new(Vec3::from_array(pos));

                let normal = parser::get_normal(frame_verts[idx].normal_index);
                v.normal = Some(normal);

                if skinwidth > 0.0 && skinheight > 0.0 {
                    let u = (eff_s as f32 + 0.5) / skinwidth;
                    let vt = (eff_t as f32 + 0.5) / skinheight;
                    v.uvs[0] = Some(Vec2::new(u, vt));
                }

                let new_idx = solid_verts.len() as u32;
                solid_verts.push(v);
                new_idx
            });
            indices.push(vert_idx);
        }
    }

    if indices.is_empty() {
        return Ok(b.build());
    }

    let mut mesh = Mesh::new("MDL Model");
    mesh.vertices = solid_verts;
    mesh.primitives.push(Primitive::triangles(indices, Some(0)));
    mesh.compute_bounds();

    let mesh_idx = b.push_mesh(mesh);
    let root = b.add_root_node("MDL Model");
    b.attach_mesh(root, mesh_idx);

    Ok(b.build())
}

// ── Minimal PNG encoder ────────────────────────────────────────────────────────
// Uses flate2 (already a workspace dependency) for deflate compression.
// CRC-32 is computed inline to avoid pulling in the crc32fast crate.

fn encode_rgba_to_png(rgba: &[u8], width: usize, height: usize) -> Vec<u8> {
    use std::io::Write;

    // Build raw filtered image data (filter byte 0 = None per row)
    let stride = width * 4;
    let mut raw = Vec::with_capacity(height * (1 + stride));
    for y in 0..height {
        raw.push(0);
        raw.extend_from_slice(&rgba[y * stride..(y + 1) * stride]);
    }

    // Deflate compress
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&raw).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut png = Vec::new();

    // PNG signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR chunk
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    write_png_chunk(&mut png, b"IHDR", &ihdr);

    // IDAT chunk
    write_png_chunk(&mut png, b"IDAT", &compressed);

    // IEND chunk
    write_png_chunk(&mut png, b"IEND", &[]);

    png
}

fn write_png_chunk(png: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    let len = data.len() as u32;
    png.extend_from_slice(&len.to_be_bytes());
    png.extend_from_slice(chunk_type);
    png.extend_from_slice(data);

    let crc = calc_crc32(chunk_type, data);
    png.extend_from_slice(&crc.to_be_bytes());
}

fn calc_crc32(type_bytes: &[u8; 4], data: &[u8]) -> u32 {
    // Build the CRC-32 table
    let mut table = [0u32; 256];
    for i in 0..256 {
        let mut crc = i as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = 0xedb88320 ^ (crc >> 1);
            } else {
                crc >>= 1;
            }
        }
        table[i] = crc;
    }

    let mut crc = 0xffffffffu32;
    for &b in type_bytes.iter().chain(data.iter()) {
        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffffffff
}
