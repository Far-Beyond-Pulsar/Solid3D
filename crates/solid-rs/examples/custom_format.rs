//! Demonstrates how a format extension crate implements [`Loader`] and
//! [`Saver`] for a hypothetical `.xyz` format and registers them with the
//! [`Registry`].
//!
//! This is the pattern followed by real format crates such as `solid-obj`,
//! `solid-gltf`, and `solid-fbx`.

use std::io::{BufRead, BufReader, Read, Write};

use solid_rs::prelude::*;
use solid_rs::traits::ReadSeek;
use glam::Vec3;

// ── Format metadata ───────────────────────────────────────────────────────────

static XYZ_FORMAT: FormatInfo = FormatInfo {
    name:         "XYZ Point Cloud",
    id:           "xyz",
    extensions:   &["xyz"],
    mime_types:   &["text/plain"],
    can_load:     true,
    can_save:     true,
    spec_version: None,
};

// ── Loader ────────────────────────────────────────────────────────────────────

/// Loader for the trivial XYZ point-cloud format.
///
/// File syntax: one `x y z` triplet per line, e.g.:
/// ```text
/// 0.0 0.0 0.0
/// 1.0 0.0 0.0
/// 0.0 1.0 0.0
/// ```
pub struct XyzLoader;

impl Loader for XyzLoader {
    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        _options: &LoadOptions,
    ) -> Result<Scene> {
        let mut builder = SceneBuilder::named("XYZ Scene");
        let mut mesh    = Mesh::new("Points");

        for (line_no, line) in BufReader::new(reader).lines().enumerate() {
            let line = line.map_err(SolidError::Io)?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let coords: Vec<f32> = line
                .split_whitespace()
                .map(|s| {
                    s.parse::<f32>().map_err(|_| {
                        SolidError::parse(format!(
                            "expected float on line {}, got {:?}", line_no + 1, s
                        ))
                    })
                })
                .collect::<Result<_>>()?;

            if coords.len() < 3 {
                return Err(SolidError::parse(format!(
                    "line {} needs at least 3 coordinates, got {}",
                    line_no + 1,
                    coords.len()
                )));
            }

            mesh.vertices.push(Vertex::new(Vec3::new(coords[0], coords[1], coords[2])));
        }

        let n = mesh.vertices.len() as u32;
        mesh.primitives = vec![Primitive::points((0..n).collect(), None)];
        mesh.compute_bounds();

        let mesh_idx = builder.push_mesh(mesh);
        let root     = builder.add_root_node("Root");
        builder.attach_mesh(root, mesh_idx);

        Ok(builder.build())
    }

    fn format_info(&self) -> &FormatInfo {
        &XYZ_FORMAT
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 32];
        let n = reader.read(&mut buf).unwrap_or(0);
        let s = std::str::from_utf8(&buf[..n]).unwrap_or("");
        // Heuristic: first non-whitespace char is a digit or '-'
        if s.trim_start().chars().next().map_or(false, |c| c.is_ascii_digit() || c == '-') {
            0.4
        } else {
            0.0
        }
    }
}

// ── Saver ─────────────────────────────────────────────────────────────────────

/// Saver for the trivial XYZ point-cloud format.
pub struct XyzSaver;

impl Saver for XyzSaver {
    fn save(
        &self,
        scene: &Scene,
        writer: &mut dyn Write,
        _options: &SaveOptions,
    ) -> Result<()> {
        writeln!(writer, "# Exported by solid-rs XyzSaver")
            .map_err(SolidError::Io)?;

        for mesh in &scene.meshes {
            for vertex in &mesh.vertices {
                let p = vertex.position;
                writeln!(writer, "{} {} {}", p.x, p.y, p.z)
                    .map_err(SolidError::Io)?;
            }
        }
        Ok(())
    }

    fn format_info(&self) -> &FormatInfo {
        &XYZ_FORMAT
    }
}

// ── Demo ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    // Register the custom format.
    let mut registry = Registry::new();
    registry
        .register_loader(XyzLoader)
        .register_saver(XyzSaver);

    println!("Registered formats:");
    for info in registry.loader_infos() {
        println!("  load  [{id}] {name}", id = info.id, name = info.name);
    }
    for info in registry.saver_infos() {
        println!("  save  [{id}] {name}", id = info.id, name = info.name);
    }

    // Build a tiny scene and round-trip it through the XYZ format in memory.
    let original = {
        let mut b = SceneBuilder::named("PointCloud");
        let mut mesh = Mesh::new("Points");
        mesh.vertices = vec![
            Vertex::new(Vec3::new(0.0, 0.0, 0.0)),
            Vertex::new(Vec3::new(1.0, 0.0, 0.0)),
            Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
            Vertex::new(Vec3::new(0.0, 0.0, 1.0)),
        ];
        let n   = mesh.vertices.len() as u32;
        mesh.primitives = vec![Primitive::points((0..n).collect(), None)];
        let idx = b.push_mesh(mesh);
        let r   = b.add_root_node("Root");
        b.attach_mesh(r, idx);
        b.build()
    };

    // Save to an in-memory buffer.
    let mut buf = Vec::new();
    XyzSaver.save(&original, &mut buf, &SaveOptions::default())?;
    let xyz_text = String::from_utf8(buf.clone()).unwrap();
    println!("\nSaved XYZ:\n{}", xyz_text);

    // Reload from that buffer.
    let loaded = XyzLoader.load(
        &mut std::io::Cursor::new(buf),
        &LoadOptions::default(),
    )?;
    println!(
        "Reloaded {} vertices",
        loaded.meshes.first().map(|m| m.vertices.len()).unwrap_or(0)
    );

    Ok(())
}
