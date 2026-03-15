//! `FbxSaver` — saves a `solid_rs::Scene` as an ASCII FBX 7.4 file.
//!
//! ASCII FBX was chosen for the saver because it is human-readable and
//! requires no separate binary serialisation infrastructure.

use std::io::Write;

use glam::{EulerRot, Vec3};

use solid_rs::prelude::*;
use solid_rs::scene::Scene;
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

fn next_id(counter: &mut i64) -> i64 {
    *counter += 1;
    *counter
}

impl<'w> FbxWriter<'w> {
    fn write_scene(&mut self, scene: &Scene) -> Result<()> {
        self.write_header()?;

        let mut id_counter: i64 = 0;
        let mesh_ids:  Vec<i64> = (0..scene.meshes.len()).map(|_| next_id(&mut id_counter)).collect();
        let mat_ids:   Vec<i64> = (0..scene.materials.len()).map(|_| next_id(&mut id_counter)).collect();
        let tex_ids:   Vec<i64> = (0..scene.textures.len()).map(|_| next_id(&mut id_counter)).collect();
        let node_ids:  Vec<i64> = (0..scene.nodes.len()).map(|_| next_id(&mut id_counter)).collect();

        // ── Definitions ──────────────────────────────────────────────────────
        let total = scene.meshes.len() + scene.materials.len()
                  + scene.textures.len() + scene.nodes.len();
        self.line("Definitions:  {")?;
        self.indent += 1;
        self.line("Version: 100")?;
        self.line(&format!("Count: {total}"))?;
        self.indent -= 1;
        self.line("}")?;
        self.blank()?;

        // ── Objects ───────────────────────────────────────────────────────────
        self.line("Objects:  {")?;
        self.indent += 1;

        for (i, mesh) in scene.meshes.iter().enumerate() {
            self.write_geometry(mesh_ids[i], mesh)?;
        }
        for (i, node) in scene.nodes.iter().enumerate() {
            self.write_model(node_ids[i], node)?;
        }
        for (i, mat) in scene.materials.iter().enumerate() {
            self.write_material(mat_ids[i], mat)?;
        }
        for (i, tex) in scene.textures.iter().enumerate() {
            // Resolve image URI from scene.images if available
            let uri = scene.images
                .get(tex.image_index)
                .and_then(|img| if let solid_rs::scene::ImageSource::Uri(u) = &img.source { Some(u.as_str()) } else { None })
                .unwrap_or("");
            self.write_texture(tex_ids[i], &tex.name, uri)?;
        }

        self.indent -= 1;
        self.line("}")?;
        self.blank()?;

        // ── Connections ───────────────────────────────────────────────────────
        self.line("Connections:  {")?;
        self.indent += 1;

        // Node position in the node Vec → its NodeId.0 value; but we mapped
        // node_ids by Vec index, so we need the Vec index of each node.
        // Build NodeId.0 → Vec-index map
        let node_id_to_vec: std::collections::HashMap<u32, usize> = scene.nodes
            .iter().enumerate().map(|(i, n)| (n.id.0, i)).collect();

        for (ni, node) in scene.nodes.iter().enumerate() {
            let nid = node_ids[ni];

            // Geometry → Model
            if let Some(mi) = node.mesh {
                self.line(&format!("C: \"OO\",{},{}", mesh_ids[mi], nid))?;
            }

            // Material → Model (via first primitive's material_index)
            if let Some(mi) = node.mesh
                .and_then(|mi| scene.meshes[mi].primitives.first())
                .and_then(|p| p.material_index)
            {
                self.line(&format!("C: \"OO\",{},{}", mat_ids[mi], nid))?;
            }

            // Model → parent (or scene root = 0)
            let parent_id = node.parent
                .and_then(|pid| node_id_to_vec.get(&pid.0))
                .map(|&vi| node_ids[vi])
                .unwrap_or(0);
            self.line(&format!("C: \"OO\",{},{}", nid, parent_id))?;
        }

        // Texture → Material (OP)
        for (mi, mat) in scene.materials.iter().enumerate() {
            let mid = mat_ids[mi];
            if let Some(tr) = &mat.base_color_texture {
                self.line(&format!(
                    "C: \"OP\",{},{},\"DiffuseColor\"", tex_ids[tr.texture_index], mid
                ))?;
            }
            if let Some(tr) = &mat.normal_texture {
                self.line(&format!(
                    "C: \"OP\",{},{},\"NormalMap\"", tex_ids[tr.texture_index], mid
                ))?;
            }
        }

        self.indent -= 1;
        self.line("}")?;
        Ok(())
    }

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

    fn write_geometry(&mut self, id: i64, mesh: &solid_rs::scene::Mesh) -> Result<()> {
        self.line(&format!(
            "Geometry: {id}, \"{}\", \"Mesh\"  {{", escape(&mesh.name)
        ))?;
        self.indent += 1;

        // Vertices
        let verts: Vec<f64> = mesh.vertices.iter()
            .flat_map(|v| [v.position.x as f64, v.position.y as f64, v.position.z as f64])
            .collect();
        self.write_f64_array("Vertices", &verts)?;

        // PolygonVertexIndex from primitives
        let mut pvi: Vec<i32> = Vec::new();
        for prim in &mesh.primitives {
            let idx = &prim.indices;
            let n   = idx.len();
            for (j, &vi) in idx.iter().enumerate() {
                if j == n - 1 { pvi.push(!(vi as i32)); } else { pvi.push(vi as i32); }
            }
        }
        self.write_i32_array("PolygonVertexIndex", &pvi)?;

        // Normals
        let normals: Vec<f64> = mesh.vertices.iter()
            .flat_map(|v| {
                let n = v.normal.unwrap_or(Vec3::Y);
                [n.x as f64, n.y as f64, n.z as f64]
            }).collect();
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
                let uv = v.uvs[0].unwrap_or_default();
                [uv.x as f64, (1.0 - uv.y) as f64] // flip V back for FBX
            }).collect();
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

    fn write_model(&mut self, id: i64, node: &solid_rs::scene::Node) -> Result<()> {
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

    fn write_material(&mut self, id: i64, mat: &solid_rs::scene::Material) -> Result<()> {
        self.line(&format!(
            "Material: {id}, \"{}\", \"\"  {{", escape(&mat.name)
        ))?;
        self.indent += 1;
        self.line("ShadingModel: \"phong\"")?;
        self.line("Properties70:  {")?;
        self.indent += 1;
        let c = mat.base_color_factor;
        let e = mat.emissive_factor;
        self.line(&format!(
            "P: \"DiffuseColor\", \"Color\", \"\", \"A\",{},{},{}", c.x, c.y, c.z
        ))?;
        self.line(&format!(
            "P: \"EmissiveColor\", \"Color\", \"\", \"A\",{},{},{}", e.x, e.y, e.z
        ))?;
        self.indent -= 1;
        self.line("}")?;
        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    fn write_texture(&mut self, id: i64, name: &str, uri: &str) -> Result<()> {
        self.line(&format!(
            "Texture: {id}, \"{}\", \"\"  {{", escape(name)
        ))?;
        self.indent += 1;
        self.line(&format!("FileName: \"{uri}\""))?;
        self.line(&format!("RelativeFilename: \"{uri}\""))?;
        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

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

    fn line(&mut self, s: &str) -> Result<()> {
        let pad = "\t".repeat(self.indent);
        writeln!(self.inner, "{pad}{s}").map_err(SolidError::Io)
    }

    fn blank(&mut self) -> Result<()> {
        writeln!(self.inner).map_err(SolidError::Io)
    }
}

fn escape(s: &str) -> String {
    s.replace('"', "\\\"")
}
