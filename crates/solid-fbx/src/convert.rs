//! FBX document → `solid_rs::Scene` conversion.
//!
//! This module walks the FBX DOM and constructs a `Scene` using
//! `SceneBuilder`.  Supported FBX features:
//!
//! * Geometry nodes → triangulated `Mesh` with positions, normals and UVs
//! * Model nodes → `Node` with transform extracted from `Properties70`
//! * Material nodes → `Material` with basic diffuse/emissive colour
//! * Texture nodes → `Texture`
//! * OO/OP connections wiring the object graph together

use std::collections::HashMap;

use glam::{EulerRot, Quat, Vec2, Vec3};

use solid_rs::scene::{
    ColorRgba, Material, Mesh, Node, PbrMetallicRoughness, Texture, TextureRef,
};
use solid_rs::geometry::{Face, Vertex};
use solid_rs::{Result, SceneBuilder, SolidError};
use solid_rs::scene::Scene;

use crate::document::{FbxDocument, FbxNode, FbxProperty};

// ── Public entry point ────────────────────────────────────────────────────────

/// Convert a parsed `FbxDocument` into a `solid_rs::Scene`.
pub(crate) fn fbx_to_scene(doc: &FbxDocument) -> Result<Scene> {
    let mut conv = Converter::new(doc);
    conv.run()
}

// ── Converter ─────────────────────────────────────────────────────────────────

struct Converter<'d> {
    doc: &'d FbxDocument,
    builder: SceneBuilder,

    /// FBX object ID → mesh index (inside the scene being built)
    geom_id_to_mesh: HashMap<i64, usize>,
    /// FBX object ID → material index
    mat_id_to_mat: HashMap<i64, usize>,
    /// FBX object ID → texture index
    tex_id_to_tex: HashMap<i64, usize>,
    /// FBX object ID → node index
    model_id_to_node: HashMap<i64, usize>,
}

impl<'d> Converter<'d> {
    fn new(doc: &'d FbxDocument) -> Self {
        Self {
            doc,
            builder: SceneBuilder::new(),
            geom_id_to_mesh:    HashMap::new(),
            mat_id_to_mat:      HashMap::new(),
            tex_id_to_tex:      HashMap::new(),
            model_id_to_node:   HashMap::new(),
        }
    }

    fn run(&mut self) -> Result<Scene> {
        // Pass 1 — objects
        if let Some(objects) = self.doc.find("Objects") {
            for child in &objects.children {
                match child.name.as_str() {
                    "Geometry" => self.process_geometry(child)?,
                    "Material" => self.process_material(child)?,
                    "Texture"  => self.process_texture(child)?,
                    "Model"    => self.process_model(child)?,
                    _ => {}
                }
            }
        }

        // Pass 2 — connections
        if let Some(conns) = self.doc.find("Connections") {
            for c in conns.children_named("C") {
                self.process_connection(c);
            }
        }

        // Collect root nodes (those with no parent after connection pass)
        let scene = self.builder.build();
        Ok(scene)
    }

    // ── Geometry ──────────────────────────────────────────────────────────────

    fn process_geometry(&mut self, node: &FbxNode) -> Result<()> {
        let id = node.id().unwrap_or(0);

        // Vertex positions
        let verts: Vec<f64> = node
            .child("Vertices")
            .and_then(|n| n.as_f64_slice())
            .map(|s| s.to_vec())
            .unwrap_or_default();

        // Polygon vertex indices (negative = end-of-polygon; real = bitwise NOT)
        let pvi: Vec<i32> = node
            .child("PolygonVertexIndex")
            .and_then(|n| n.as_i32_slice())
            .map(|s| s.to_vec())
            .unwrap_or_default();

        if verts.is_empty() || pvi.is_empty() {
            return Ok(());
        }

        // Normals (ByPolygonVertex preferred)
        let normals = self.extract_layer_f64(node, "LayerElementNormal", "Normals");
        let normal_mode = self.extract_mapping_mode(node, "LayerElementNormal");

        // UVs
        let uvs = self.extract_layer_f64(node, "LayerElementUV", "UV");
        let uv_mode = self.extract_mapping_mode(node, "LayerElementUV");

        // Fan-triangulate the polygon list
        let mut vertices: Vec<Vertex>  = Vec::new();
        let mut faces:    Vec<Face>    = Vec::new();

        let mut poly_start = 0usize;
        let mut flat_idx   = 0usize; // index into ByPolygonVertex arrays

        for (i, &raw_idx) in pvi.iter().enumerate() {
            let is_last = raw_idx < 0;
            let vert_idx = if is_last { (!raw_idx) as usize } else { raw_idx as usize };

            // Position
            let px = *verts.get(vert_idx * 3    ).unwrap_or(&0.0) as f32;
            let py = *verts.get(vert_idx * 3 + 1).unwrap_or(&0.0) as f32;
            let pz = *verts.get(vert_idx * 3 + 2).unwrap_or(&0.0) as f32;

            // Normal
            let sample_idx = match normal_mode {
                MappingMode::ByPolygonVertex => flat_idx,
                MappingMode::ByVertex        => vert_idx,
                _ => flat_idx,
            };
            let nx = *normals.get(sample_idx * 3    ).unwrap_or(&0.0) as f32;
            let ny = *normals.get(sample_idx * 3 + 1).unwrap_or(&0.0) as f32;
            let nz = *normals.get(sample_idx * 3 + 2).unwrap_or(&0.0) as f32;

            // UV
            let uv_si = match uv_mode {
                MappingMode::ByPolygonVertex => flat_idx,
                MappingMode::ByVertex        => vert_idx,
                _ => flat_idx,
            };
            let u = *uvs.get(uv_si * 2    ).unwrap_or(&0.0) as f32;
            let v = *uvs.get(uv_si * 2 + 1).unwrap_or(&0.0) as f32;

            let v = Vertex::new(
                Vec3::new(px, py, pz),
                Some(Vec3::new(nx, ny, nz)),
                Some(Vec2::new(u, 1.0 - v)), // FBX V is flipped vs OpenGL
                None,
            );
            vertices.push(v);
            flat_idx += 1;

            if is_last {
                // Fan-triangulate the polygon [poly_start .. i] (inclusive)
                let poly_len = i - poly_start + 1;
                if poly_len >= 3 {
                    for fi in 1..poly_len - 1 {
                        faces.push(Face::triangle(
                            poly_start as u32,
                            (poly_start + fi) as u32,
                            (poly_start + fi + 1) as u32,
                        ));
                    }
                }
                poly_start = i + 1;
            }
        }

        let name = node.object_name()
            .unwrap_or("Geometry")
            .split('\x00').next().unwrap_or("Geometry")
            .to_owned();

        let mesh_idx = self.builder.scene.meshes.len();
        let mut mesh = Mesh::new(&name);
        mesh.vertices = vertices;
        mesh.faces    = faces;
        self.builder.scene.meshes.push(mesh);
        self.geom_id_to_mesh.insert(id, mesh_idx);

        Ok(())
    }

    fn extract_layer_f64(&self, geo: &FbxNode, layer: &str, key: &str) -> Vec<f64> {
        geo.child(layer)
            .and_then(|l| l.child(key))
            .and_then(|n| n.as_f64_slice())
            .map(|s| s.to_vec())
            .unwrap_or_default()
    }

    fn extract_mapping_mode(&self, geo: &FbxNode, layer: &str) -> MappingMode {
        geo.child(layer)
            .and_then(|l| l.child("MappingInformationType"))
            .and_then(|n| n.as_str())
            .map(MappingMode::from_str)
            .unwrap_or(MappingMode::ByPolygonVertex)
    }

    // ── Material ──────────────────────────────────────────────────────────────

    fn process_material(&mut self, node: &FbxNode) -> Result<()> {
        let id = node.id().unwrap_or(0);
        let name = node.object_name()
            .unwrap_or("Material")
            .split('\x00').next().unwrap_or("Material")
            .to_owned();

        let mut mat = Material::new(&name);

        // Extract diffuse colour from Properties70
        if let Some(props) = node.child("Properties70") {
            for p in props.children_named("P") {
                let Some(pname) = p.properties.first().and_then(FbxProperty::as_str) else { continue };
                match pname {
                    "DiffuseColor" | "Diffuse" => {
                        let r = p.properties.get(4).and_then(FbxProperty::as_f64).unwrap_or(0.8) as f32;
                        let g = p.properties.get(5).and_then(FbxProperty::as_f64).unwrap_or(0.8) as f32;
                        let b = p.properties.get(6).and_then(FbxProperty::as_f64).unwrap_or(0.8) as f32;
                        mat.pbr = PbrMetallicRoughness {
                            base_color_factor: ColorRgba::new(r, g, b, 1.0),
                            ..mat.pbr
                        };
                    }
                    "EmissiveColor" | "Emissive" => {
                        let r = p.properties.get(4).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                        let g = p.properties.get(5).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                        let b = p.properties.get(6).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                        mat.emissive_factor = [r, g, b];
                    }
                    _ => {}
                }
            }
        }

        let idx = self.builder.scene.materials.len();
        self.builder.scene.materials.push(mat);
        self.mat_id_to_mat.insert(id, idx);
        Ok(())
    }

    // ── Texture ───────────────────────────────────────────────────────────────

    fn process_texture(&mut self, node: &FbxNode) -> Result<()> {
        let id   = node.id().unwrap_or(0);
        let name = node.object_name()
            .unwrap_or("Texture")
            .split('\x00').next().unwrap_or("Texture")
            .to_owned();

        let path = node.child("FileName")
            .or_else(|| node.child("RelativeFilename"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_owned();

        let tex_idx = self.builder.scene.textures.len();
        let mut tex = Texture::new(&name, 0); // source_index=0 placeholder
        tex.uri    = Some(path);
        self.builder.scene.textures.push(tex);
        self.tex_id_to_tex.insert(id, tex_idx);
        Ok(())
    }

    // ── Model (node) ──────────────────────────────────────────────────────────

    fn process_model(&mut self, node: &FbxNode) -> Result<()> {
        let id = node.id().unwrap_or(0);
        let name = node.object_name()
            .unwrap_or("Node")
            .split('\x00').next().unwrap_or("Node")
            .to_owned();

        let mut translation = Vec3::ZERO;
        let mut rotation_deg = Vec3::ZERO;
        let mut scale        = Vec3::ONE;

        if let Some(props) = node.child("Properties70") {
            for p in props.children_named("P") {
                let Some(pname) = p.properties.first().and_then(FbxProperty::as_str) else { continue };
                match pname {
                    "LclTranslation" | "Lcl Translation" => {
                        translation.x = p.properties.get(4).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                        translation.y = p.properties.get(5).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                        translation.z = p.properties.get(6).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                    }
                    "LclRotation" | "Lcl Rotation" => {
                        rotation_deg.x = p.properties.get(4).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                        rotation_deg.y = p.properties.get(5).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                        rotation_deg.z = p.properties.get(6).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32;
                    }
                    "LclScaling" | "Lcl Scaling" => {
                        scale.x = p.properties.get(4).and_then(FbxProperty::as_f64).unwrap_or(1.0) as f32;
                        scale.y = p.properties.get(5).and_then(FbxProperty::as_f64).unwrap_or(1.0) as f32;
                        scale.z = p.properties.get(6).and_then(FbxProperty::as_f64).unwrap_or(1.0) as f32;
                    }
                    _ => {}
                }
            }
        }

        // FBX uses XYZ Euler order in degrees
        let rot = Quat::from_euler(
            EulerRot::XYZ,
            rotation_deg.x.to_radians(),
            rotation_deg.y.to_radians(),
            rotation_deg.z.to_radians(),
        );

        let node_idx = self.builder.add_root_node(
            solid_rs::scene::Node::new(&name)
        );
        {
            let n = &mut self.builder.scene.nodes[node_idx];
            n.transform.translation = translation;
            n.transform.rotation    = rot;
            n.transform.scale       = scale;
        }

        self.model_id_to_node.insert(id, node_idx);
        Ok(())
    }

    // ── Connections ───────────────────────────────────────────────────────────

    fn process_connection(&mut self, c: &FbxNode) {
        let ctype  = c.properties.first().and_then(FbxProperty::as_str).unwrap_or("");
        let src_id = c.properties.get(1).and_then(FbxProperty::as_i64).unwrap_or(0);
        let dst_id = c.properties.get(2).and_then(FbxProperty::as_i64).unwrap_or(0);
        let prop   = c.properties.get(3).and_then(FbxProperty::as_str).unwrap_or("");

        match ctype {
            "OO" => {
                // Geometry → Model
                if let (Some(&mesh_idx), Some(&model_idx)) = (
                    self.geom_id_to_mesh.get(&src_id),
                    self.model_id_to_node.get(&dst_id),
                ) {
                    self.builder.scene.nodes[model_idx].mesh_index = Some(mesh_idx);
                    return;
                }

                // Material → Model
                if let (Some(&mat_idx), Some(&model_idx)) = (
                    self.mat_id_to_mat.get(&src_id),
                    self.model_id_to_node.get(&dst_id),
                ) {
                    self.builder.scene.nodes[model_idx].material_index = Some(mat_idx);
                    return;
                }

                // Model → Model (parent–child)
                if let (Some(&child_node_idx), Some(&parent_node_idx)) = (
                    self.model_id_to_node.get(&src_id),
                    self.model_id_to_node.get(&dst_id),
                ) {
                    // Remove from root scene list if present
                    self.builder.scene.root_nodes.retain(|&r| r != child_node_idx);
                    // Wire parent ↔ child
                    self.builder.scene.nodes[child_node_idx].parent = Some(parent_node_idx);
                    self.builder.scene.nodes[parent_node_idx].children.push(child_node_idx);
                    return;
                }

                // Model → scene root (dst_id == 0)
                if dst_id == 0 {
                    if let Some(&node_idx) = self.model_id_to_node.get(&src_id) {
                        if !self.builder.scene.root_nodes.contains(&node_idx) {
                            self.builder.scene.root_nodes.push(node_idx);
                        }
                    }
                }
            }
            "OP" => {
                // Texture → Material via property name
                if let (Some(&tex_idx), Some(&mat_idx)) = (
                    self.tex_id_to_tex.get(&src_id),
                    self.mat_id_to_mat.get(&dst_id),
                ) {
                    let tex_ref = TextureRef { index: tex_idx, tex_coord: 0 };
                    let mat = &mut self.builder.scene.materials[mat_idx];
                    match prop {
                        "DiffuseColor" | "Diffuse" => {
                            mat.pbr.base_color_texture = Some(tex_ref);
                        }
                        "NormalMap" | "Bump" => {
                            mat.normal_texture = Some(tex_ref);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

// ── MappingMode ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum MappingMode {
    ByPolygonVertex,
    ByVertex,
    Other,
}

impl MappingMode {
    fn from_str(s: &str) -> Self {
        match s {
            "ByPolygonVertex" => MappingMode::ByPolygonVertex,
            "ByVertex" | "ByVertice" => MappingMode::ByVertex,
            _ => MappingMode::Other,
        }
    }
}
