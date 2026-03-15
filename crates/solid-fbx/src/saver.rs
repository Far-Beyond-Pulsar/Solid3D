//! `FbxSaver` — saves a `solid_rs::Scene` as an ASCII FBX 7.4 file.
//!
//! ASCII FBX was chosen for the saver because it is human-readable and
//! requires no separate binary serialisation infrastructure.  Binary FBX
//! round-trips can be achieved by loading from binary and writing to a
//! different path.

use std::io::Write;

use glam::{EulerRot, Vec3};

use solid_rs::prelude::*;
use solid_rs::scene::{Scene, Light, Camera, Material, Texture};
use solid_rs::geometry::Vertex;
use solid_rs::{Result, SolidError};

use crate::FBX_FORMAT;

/// Saves a `Scene` as ASCII FBX 7.4.
pub struct FbxSaver;

impl Saver for FbxSaver {
    fn format_info(&self) -> &FormatInfo {
        &FBX_FORMAT
    }

    fn save(
        &self,
        scene: &Scene,
        writer: &mut dyn Write,
        _options: &SaveOptions,
    ) -> Result<()> {
        let mut w = FbxWriter { inner: writer, indent: 0 };
        w.write_scene(scene)
    }
}

// ── Writer ────────────────────────────────────────────────────────────────────

struct FbxWriter<'w> {
    inner:  &'w mut dyn Write,
    indent: usize,
}

/// Monotonically increasing ID counter (scene root = 0, objects start at 1).
fn next_id(counter: &mut i64) -> i64 {
    *counter += 1;
    *counter
}

impl<'w> FbxWriter<'w> {
    fn write_scene(&mut self, scene: &Scene) -> Result<()> {
        self.write_header()?;

        // Assign unique IDs to every object
        let mut id_counter: i64 = 0;
        let mesh_ids:     Vec<i64> = (0..scene.meshes.len()).map(|_| next_id(&mut id_counter)).collect();
        let mat_ids:      Vec<i64> = (0..scene.materials.len()).map(|_| next_id(&mut id_counter)).collect();
        let tex_ids:      Vec<i64> = (0..scene.textures.len()).map(|_| next_id(&mut id_counter)).collect();
        let node_ids:     Vec<i64> = (0..scene.nodes.len()).map(|_| next_id(&mut id_counter)).collect();

        // ── Definitions ──────────────────────────────────────────────────────
        self.line("Definitions:  {")?;
        self.indent += 1;
        self.line("Version: 100")?;
        let total = scene.meshes.len() + scene.materials.len()
                  + scene.textures.len() + scene.nodes.len();
        self.line(&format!("Count: {total}"))?;
        self.indent -= 1;
        self.line("}")?;
        self.blank()?;

        // ── Objects ───────────────────────────────────────────────────────────
        self.line("Objects:  {")?;
        self.indent += 1;

        // Geometries
        for (i, mesh) in scene.meshes.iter().enumerate() {
            self.write_geometry(mesh_ids[i], mesh)?;
        }

        // Models (nodes)
        for (i, node) in scene.nodes.iter().enumerate() {
            self.write_model(node_ids[i], node, scene)?;
        }

        // Materials
        for (i, mat) in scene.materials.iter().enumerate() {
            self.write_material(mat_ids[i], mat)?;
        }

        // Textures
        for (i, tex) in scene.textures.iter().enumerate() {
            self.write_texture(tex_ids[i], tex)?;
        }

        self.indent -= 1;
        self.line("}")?;
        self.blank()?;

        // ── Connections ───────────────────────────────────────────────────────
        self.line("Connections:  {")?;
        self.indent += 1;

        for (ni, node) in scene.nodes.iter().enumerate() {
            let nid = node_ids[ni];

            // Geometry → Model
            if let Some(mi) = node.mesh_index {
                self.line(&format!(
                    "C: \"OO\",{},{}", mesh_ids[mi], nid
                ))?;
            }

            // Material → Model
            if let Some(mi) = node.material_index {
                self.line(&format!(
                    "C: \"OO\",{},{}", mat_ids[mi], nid
                ))?;
            }

            // Model → parent (or root)
            let parent_id = node.parent
                .map(|pi| node_ids[pi])
                .unwrap_or(0);
            self.line(&format!("C: \"OO\",{},{}", nid, parent_id))?;
        }

        // Texture → Material (OP connections)
        for (mi, mat) in scene.materials.iter().enumerate() {
            let mid = mat_ids[mi];
            if let Some(tr) = mat.pbr.base_color_texture {
                self.line(&format!(
                    "C: \"OP\",{},{},\"DiffuseColor\"", tex_ids[tr.index], mid
                ))?;
            }
            if let Some(tr) = mat.normal_texture {
                self.line(&format!(
                    "C: \"OP\",{},{},\"NormalMap\"", tex_ids[tr.index], mid
                ))?;
            }
        }

        self.indent -= 1;
        self.line("}")?;

        Ok(())
    }

    // ── Header ────────────────────────────────────────────────────────────────

    fn write_header(&mut self) -> Result<()> {
        self.line("; FBX 7.4.0 project file")?;
        self.line("; Saved by solid-fbx")?;
        self.blank()?;
        self.line("FBXHeaderExtension:  {")?;
        self.indent += 1;
        self.line("FBXHeaderVersion: 1003")?;
        self.line("FBXVersion: 7400")?;
        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    // ── Geometry ──────────────────────────────────────────────────────────────

    fn write_geometry(&mut self, id: i64, mesh: &solid_rs::scene::Mesh) -> Result<()> {
        self.line(&format!(
            "Geometry: {id}, \"{}\", \"Mesh\"  {{", escape(&mesh.name)
        ))?;
        self.indent += 1;

        // Vertices
        let mut verts = Vec::with_capacity(mesh.vertices.len() * 3);
        for v in &mesh.vertices {
            verts.push(v.position.x as f64);
            verts.push(v.position.y as f64);
            verts.push(v.position.z as f64);
        }
        self.write_f64_array("Vertices", &verts)?;

        // PolygonVertexIndex — every face ends with a negated (+1) index
        let mut pvi: Vec<i32> = Vec::new();
        for face in &mesh.faces {
            let n = face.indices.len();
            for (j, &vi) in face.indices.iter().enumerate() {
                if j == n - 1 {
                    pvi.push(!(vi as i32));
                } else {
                    pvi.push(vi as i32);
                }
            }
        }
        self.write_i32_array("PolygonVertexIndex", &pvi)?;

        // Normals
        let normals: Vec<f64> = mesh.vertices.iter()
            .flat_map(|v| {
                let n = v.normal.unwrap_or(Vec3::Y);
                [n.x as f64, n.y as f64, n.z as f64]
            })
            .collect();
        if !normals.is_empty() {
            self.line("LayerElementNormal: 0 {")?;
            self.indent += 1;
            self.line("MappingInformationType: \"ByPolygonVertex\"")?;
            self.line("ReferenceInformationType: \"Direct\"")?;
            self.write_f64_array("Normals", &normals)?;
            self.indent -= 1;
            self.line("}")?;
        }

        // UVs
        let uvs: Vec<f64> = mesh.vertices.iter()
            .flat_map(|v| {
                let uv = v.uv0.unwrap_or_default();
                [uv.x as f64, (1.0 - uv.y) as f64] // flip V back for FBX
            })
            .collect();
        if !uvs.is_empty() {
            self.line("LayerElementUV: 0 {")?;
            self.indent += 1;
            self.line("MappingInformationType: \"ByPolygonVertex\"")?;
            self.line("ReferenceInformationType: \"Direct\"")?;
            self.write_f64_array("UV", &uvs)?;
            self.indent -= 1;
            self.line("}")?;
        }

        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    // ── Model ─────────────────────────────────────────────────────────────────

    fn write_model(&mut self, id: i64, node: &solid_rs::scene::Node, _scene: &Scene) -> Result<()> {
        self.line(&format!(
            "Model: {id}, \"{}\", \"Null\"  {{", escape(&node.name)
        ))?;
        self.indent += 1;

        self.line("Version: 232")?;

        let t = &node.transform;
        let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);

        self.line("Properties70:  {")?;
        self.indent += 1;

        self.line(&format!(
            "P: \"LclTranslation\", \"LclTranslation\", \"\", \"A\",{},{},{}",
            t.translation.x, t.translation.y, t.translation.z
        ))?;
        self.line(&format!(
            "P: \"LclRotation\", \"LclRotation\", \"\", \"A\",{},{},{}",
            rx.to_degrees(), ry.to_degrees(), rz.to_degrees()
        ))?;
        self.line(&format!(
            "P: \"LclScaling\", \"LclScaling\", \"\", \"A\",{},{},{}",
            t.scale.x, t.scale.y, t.scale.z
        ))?;

        self.indent -= 1;
        self.line("}")?;

        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    // ── Material ──────────────────────────────────────────────────────────────

    fn write_material(&mut self, id: i64, mat: &Material) -> Result<()> {
        self.line(&format!(
            "Material: {id}, \"{}\", \"\"  {{", escape(&mat.name)
        ))?;
        self.indent += 1;

        let c = &mat.pbr.base_color_factor;
        let [er, eg, eb] = mat.emissive_factor;

        self.line("ShadingModel: \"phong\"")?;
        self.line("Properties70:  {")?;
        self.indent += 1;
        self.line(&format!(
            "P: \"DiffuseColor\", \"Color\", \"\", \"A\",{},{},{}", c.r, c.g, c.b
        ))?;
        self.line(&format!(
            "P: \"EmissiveColor\", \"Color\", \"\", \"A\",{},{},{}", er, eg, eb
        ))?;
        self.indent -= 1;
        self.line("}")?;

        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    // ── Texture ───────────────────────────────────────────────────────────────

    fn write_texture(&mut self, id: i64, tex: &Texture) -> Result<()> {
        self.line(&format!(
            "Texture: {id}, \"{}\", \"\"  {{", escape(&tex.name)
        ))?;
        self.indent += 1;

        let uri = tex.uri.as_deref().unwrap_or("");
        self.line(&format!("FileName: \"{uri}\""))?;
        self.line(&format!("RelativeFilename: \"{uri}\""))?;

        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    // ── Array helpers ─────────────────────────────────────────────────────────

    fn write_f64_array(&mut self, name: &str, data: &[f64]) -> Result<()> {
        self.line(&format!("{name}: *{} {{", data.len()))?;
        self.indent += 1;
        let items: Vec<String> = data.iter().map(|v| format!("{v}")).collect();
        self.line(&format!("a: {}", items.join(",")))?;
        self.indent -= 1;
        self.line("}")
    }

    fn write_i32_array(&mut self, name: &str, data: &[i32]) -> Result<()> {
        self.line(&format!("{name}: *{} {{", data.len()))?;
        self.indent += 1;
        let items: Vec<String> = data.iter().map(|v| format!("{v}")).collect();
        self.line(&format!("a: {}", items.join(",")))?;
        self.indent -= 1;
        self.line("}")
    }

    // ── Low-level I/O ─────────────────────────────────────────────────────────

    fn line(&mut self, s: &str) -> Result<()> {
        let pad = "\t".repeat(self.indent);
        writeln!(self.inner, "{pad}{s}").map_err(SolidError::Io)
    }

    fn blank(&mut self) -> Result<()> {
        writeln!(self.inner).map_err(SolidError::Io)
    }
}

/// Escape a string for embedding in an FBX identifier.
fn escape(s: &str) -> String {
    s.replace('"', "\\\"")
}
