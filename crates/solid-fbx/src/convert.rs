//! FBX document → `solid_rs::Scene` conversion.
//!
//! This module walks the FBX DOM and constructs a `Scene` using
//! `SceneBuilder`.  Supported FBX features:
//!
//! * Geometry nodes → triangulated `Mesh` with positions, normals and UVs
//! * Model nodes → `Node` with transform extracted from `Properties70`
//! * Material nodes → `Material` with basic diffuse/emissive colour
//! * Texture nodes → `Texture` + backing `Image`
//! * OO/OP connections wiring the object graph together

use std::collections::HashMap;

use glam::{EulerRot, Quat, Vec2, Vec3, Vec4};

use solid_rs::builder::SceneBuilder;
use solid_rs::geometry::{Primitive, Vertex};
use solid_rs::scene::{Image, ImageSource, Material, Mesh, NodeId, TextureRef, Texture};
use solid_rs::{Result, SolidError};
use solid_rs::scene::Scene;

use crate::document::{FbxDocument, FbxNode, FbxProperty};

// ── Public entry point ────────────────────────────────────────────────────────

/// Convert a parsed `FbxDocument` into a `solid_rs::Scene`.
pub(crate) fn fbx_to_scene(doc: &FbxDocument) -> Result<Scene> {
    let mut conv = Converter::new(doc);
    conv.run()
}

// ── Intermediate types ────────────────────────────────────────────────────────

/// Extracted geometry — built in pass 1, pushed to the scene in pass 2.
struct RawGeom {
    fbx_id:   i64,
    mesh:     Mesh,
}

/// Extracted material — built in pass 1, pushed to scene in pass 2.
struct RawMat {
    fbx_id:   i64,
    material: Material,
}

/// Extracted texture/image pair.
struct RawTex {
    fbx_id:    i64,
    image_uri: String,
    name:      String,
}

/// Extracted model (node) — parenting resolved in pass 2.
struct RawModel {
    fbx_id:      i64,
    name:        String,
    translation: Vec3,
    rotation:    Quat,
    scale:       Vec3,
}

// ── Converter ─────────────────────────────────────────────────────────────────

struct Converter<'d> {
    doc: &'d FbxDocument,

    // Pass-1 intermediates
    geoms:  Vec<RawGeom>,
    mats:   Vec<RawMat>,
    texs:   Vec<RawTex>,
    models: Vec<RawModel>,

    // Pass-1 fbxID → intermediate vec index
    geom_fbx:  HashMap<i64, usize>,
    mat_fbx:   HashMap<i64, usize>,
    tex_fbx:   HashMap<i64, usize>,
    model_fbx: HashMap<i64, usize>,

    // Pass-2 connections (src_id, dst_id, property_name)
    oo_conns: Vec<(i64, i64)>,
    op_conns: Vec<(i64, i64, String)>,
}

impl<'d> Converter<'d> {
    fn new(doc: &'d FbxDocument) -> Self {
        Self {
            doc,
            geoms: Vec::new(), mats: Vec::new(), texs: Vec::new(), models: Vec::new(),
            geom_fbx: HashMap::new(), mat_fbx: HashMap::new(),
            tex_fbx: HashMap::new(), model_fbx: HashMap::new(),
            oo_conns: Vec::new(), op_conns: Vec::new(),
        }
    }

    fn run(&mut self) -> Result<Scene> {
        // ── Pass 1: extract objects ───────────────────────────────────────────
        if let Some(objects) = self.doc.find("Objects") {
            for child in &objects.children {
                match child.name.as_str() {
                    "Geometry" => self.extract_geometry(child)?,
                    "Material" => self.extract_material(child),
                    "Texture"  => self.extract_texture(child),
                    "Model"    => self.extract_model(child),
                    _ => {}
                }
            }
        }

        // ── Pass 1b: gather connections ───────────────────────────────────────
        if let Some(conns) = self.doc.find("Connections") {
            for c in conns.children_named("C") {
                let ctype  = c.properties.first().and_then(FbxProperty::as_str).unwrap_or("");
                let src_id = c.properties.get(1).and_then(FbxProperty::as_i64).unwrap_or(0);
                let dst_id = c.properties.get(2).and_then(FbxProperty::as_i64).unwrap_or(0);
                let prop   = c.properties.get(3).and_then(FbxProperty::as_str).unwrap_or("").to_owned();
                match ctype {
                    "OO" => self.oo_conns.push((src_id, dst_id)),
                    "OP" => self.op_conns.push((src_id, dst_id, prop)),
                    _ => {}
                }
            }
        }

        // ── Pass 2: build scene via SceneBuilder ──────────────────────────────
        let mut b = SceneBuilder::new();

        // Push images for textures
        let mut tex_image_map: Vec<usize> = Vec::with_capacity(self.texs.len()); // tex_idx → image_idx
        for raw in &self.texs {
            let img = Image {
                name:       raw.name.clone(),
                source:     ImageSource::Uri(raw.image_uri.clone()),
                extensions: Default::default(),
            };
            tex_image_map.push(b.push_image(img));
        }

        // Push textures
        let mut tex_scene_idxs: Vec<usize> = Vec::with_capacity(self.texs.len());
        for (i, raw) in self.texs.iter().enumerate() {
            let tex = Texture::new(&raw.name, tex_image_map[i]);
            tex_scene_idxs.push(b.push_texture(tex));
        }

        // Apply OP connections (texture → material property) before pushing materials
        // We track which material has which textures.
        // OP: src=texture, dst=material, prop=channel
        let mut mat_diffuse_tex:  HashMap<usize /* mat raw idx */, usize /* tex raw idx */> = HashMap::new();
        let mut mat_normal_tex:   HashMap<usize, usize> = HashMap::new();
        for (src_id, dst_id, prop) in &self.op_conns {
            if let (Some(&ti), Some(&mi)) = (self.tex_fbx.get(src_id), self.mat_fbx.get(dst_id)) {
                match prop.as_str() {
                    "DiffuseColor" | "Diffuse" => { mat_diffuse_tex.insert(mi, ti); }
                    "NormalMap" | "Bump"       => { mat_normal_tex.insert(mi, ti); }
                    _ => {}
                }
            }
        }

        // Push materials, applying texture references
        let mut mat_scene_idxs: Vec<usize> = Vec::with_capacity(self.mats.len());
        for (i, raw) in self.mats.iter().enumerate() {
            let mut mat = raw.material.clone();
            if let Some(&ti) = mat_diffuse_tex.get(&i) {
                mat.base_color_texture = Some(TextureRef::new(tex_scene_idxs[ti]));
            }
            if let Some(&ti) = mat_normal_tex.get(&i) {
                mat.normal_texture = Some(TextureRef::new(tex_scene_idxs[ti]));
            }
            mat_scene_idxs.push(b.push_material(mat));
        }

        // Determine geometry→material OO mapping
        // OO: src=geom, dst=model; src=material, dst=model — need to match via model
        // Build model → materials mapping (FBX IDs)
        let mut model_to_mats: HashMap<i64, Vec<i64>> = HashMap::new();
        let mut model_to_geom: HashMap<i64, i64>      = HashMap::new();
        let mut model_to_parent: HashMap<i64, i64>    = HashMap::new();
        for &(src_id, dst_id) in &self.oo_conns {
            if self.geom_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_geom.insert(dst_id, src_id);
            } else if self.mat_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_mats.entry(dst_id).or_default().push(src_id);
            } else if self.model_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_parent.insert(src_id, dst_id);
            }
        }

        // Push meshes with material indices already set on primitives
        let mut geom_scene_idxs: Vec<usize> = vec![usize::MAX; self.geoms.len()];
        // We'll push each geometry when we know its owning model's material.
        // Build a geom_fbx_id → model_fbx_id reverse map from model_to_geom
        let geom_to_model: HashMap<i64, i64> = model_to_geom.iter()
            .map(|(&model_id, &geom_id)| (geom_id, model_id))
            .collect();

        for (ri, raw) in self.geoms.iter().enumerate() {
            let first_mat_scene_idx = geom_to_model.get(&raw.fbx_id)
                .and_then(|mid| model_to_mats.get(mid))
                .and_then(|mids| mids.first())
                .and_then(|fbx_mid| self.mat_fbx.get(fbx_mid))
                .map(|&mat_ri| mat_scene_idxs[mat_ri]);

            let mut mesh = raw.mesh.clone();
            for prim in &mut mesh.primitives {
                prim.material_index = first_mat_scene_idx;
            }
            geom_scene_idxs[ri] = b.push_mesh(mesh);
        }

        // Build node creation order: roots first
        // Topological sort: nodes with no parent in model_to_parent come first
        let model_fbx_ids: Vec<i64> = self.models.iter().map(|m| m.fbx_id).collect();
        let mut created_nodes: HashMap<i64, NodeId> = HashMap::new();

        // Iteratively add nodes whose parents are already created
        let mut queue: Vec<i64> = model_fbx_ids.iter()
            .filter(|id| !model_to_parent.contains_key(*id))
            .cloned()
            .collect();
        let mut remaining: Vec<i64> = model_fbx_ids.iter()
            .filter(|id| model_to_parent.contains_key(*id))
            .cloned()
            .collect();

        loop {
            let mut progress = false;
            let mut still_remaining = Vec::new();
            for id in remaining.drain(..) {
                let parent_fbx = model_to_parent[&id];
                if let Some(&parent_node_id) = created_nodes.get(&parent_fbx) {
                    queue.push(id);
                    progress = true;
                } else {
                    still_remaining.push(id);
                }
            }
            remaining = still_remaining;
            if queue.is_empty() && remaining.is_empty() { break; }
            if !queue.is_empty() {
                for id in queue.drain(..) {
                    let raw_idx = self.model_fbx[&id];
                    let raw = &self.models[raw_idx];
                    let node_id = if let Some(&parent_fbx) = model_to_parent.get(&id) {
                        if let Some(&parent_nid) = created_nodes.get(&parent_fbx) {
                            b.add_child_node(parent_nid, &raw.name)
                        } else {
                            b.add_root_node(&raw.name)
                        }
                    } else {
                        b.add_root_node(&raw.name)
                    };
                    b.set_transform(node_id, solid_rs::geometry::Transform {
                        translation: raw.translation,
                        rotation:    raw.rotation,
                        scale:       raw.scale,
                    });

                    // Attach geometry
                    if let Some(&geom_fbx_id) = model_to_geom.get(&id) {
                        let geom_raw_idx = self.geom_fbx[&geom_fbx_id];
                        let mesh_scene_idx = geom_scene_idxs[geom_raw_idx];
                        b.attach_mesh(node_id, mesh_scene_idx);
                    }

                    created_nodes.insert(id, node_id);
                }
            } else if !remaining.is_empty() {
                // Break cycle: add remaining as roots
                for id in remaining.drain(..) {
                    let raw_idx = self.model_fbx[&id];
                    let raw = &self.models[raw_idx];
                    let node_id = b.add_root_node(&raw.name);
                    b.set_transform(node_id, solid_rs::geometry::Transform {
                        translation: raw.translation,
                        rotation:    raw.rotation,
                        scale:       raw.scale,
                    });
                    created_nodes.insert(id, node_id);
                }
                break;
            } else {
                break;
            }
            if !progress { break; }
        }

        Ok(b.build())
    }

    // ── Pass 1: object extractors ─────────────────────────────────────────────

    fn extract_geometry(&mut self, node: &FbxNode) -> Result<()> {
        let id = node.id().unwrap_or(0);
        let name = fbx_object_name(node);

        let verts: Vec<f64> = node.child("Vertices")
            .and_then(|n| n.as_f64_slice()).map(|s| s.to_vec()).unwrap_or_default();
        let pvi: Vec<i32>   = node.child("PolygonVertexIndex")
            .and_then(|n| n.as_i32_slice()).map(|s| s.to_vec()).unwrap_or_default();

        if verts.is_empty() || pvi.is_empty() { return Ok(()); }

        let normals   = extract_f64_layer(node, "LayerElementNormal", "Normals");
        let uvs       = extract_f64_layer(node, "LayerElementUV", "UV");
        let norm_mode = extract_mapping_mode(node, "LayerElementNormal");
        let uv_mode   = extract_mapping_mode(node, "LayerElementUV");

        let mut vertices:   Vec<Vertex> = Vec::new();
        let mut tri_indices: Vec<u32>   = Vec::new();
        let mut poly_start  = 0usize;
        let mut flat_idx    = 0usize;

        for (i, &raw_idx) in pvi.iter().enumerate() {
            let is_last  = raw_idx < 0;
            let vert_idx = if is_last { (!raw_idx) as usize } else { raw_idx as usize };

            let px = verts.get(vert_idx*3  ).copied().unwrap_or(0.0) as f32;
            let py = verts.get(vert_idx*3+1).copied().unwrap_or(0.0) as f32;
            let pz = verts.get(vert_idx*3+2).copied().unwrap_or(0.0) as f32;

            let ns = match norm_mode { MappingMode::ByVertex => vert_idx, _ => flat_idx };
            let nx = normals.get(ns*3  ).copied().unwrap_or(0.0) as f32;
            let ny = normals.get(ns*3+1).copied().unwrap_or(0.0) as f32;
            let nz = normals.get(ns*3+2).copied().unwrap_or(0.0) as f32;

            let us = match uv_mode { MappingMode::ByVertex => vert_idx, _ => flat_idx };
            let u  = uvs.get(us*2  ).copied().unwrap_or(0.0) as f32;
            let v  = uvs.get(us*2+1).copied().unwrap_or(0.0) as f32;

            let vtx = Vertex::new(Vec3::new(px, py, pz))
                .with_normal(Vec3::new(nx, ny, nz))
                .with_uv(Vec2::new(u, 1.0 - v)); // flip V for OpenGL

            vertices.push(vtx);
            flat_idx += 1;

            if is_last {
                let poly_len = i - poly_start + 1;
                for fi in 1..poly_len.saturating_sub(1) {
                    tri_indices.push(poly_start as u32);
                    tri_indices.push((poly_start + fi) as u32);
                    tri_indices.push((poly_start + fi + 1) as u32);
                }
                poly_start = i + 1;
            }
        }

        let mut mesh = Mesh::new(&name);
        mesh.vertices  = vertices;
        mesh.primitives = vec![Primitive::triangles(tri_indices, None)];

        let idx = self.geoms.len();
        self.geom_fbx.insert(id, idx);
        self.geoms.push(RawGeom { fbx_id: id, mesh });
        Ok(())
    }

    fn extract_material(&mut self, node: &FbxNode) {
        let id   = node.id().unwrap_or(0);
        let name = fbx_object_name(node);

        let mut mat = Material::new(&name);

        if let Some(props) = node.child("Properties70") {
            for p in props.children_named("P") {
                let pname = match p.properties.first().and_then(FbxProperty::as_str) {
                    Some(s) => s,
                    None => continue,
                };
                match pname {
                    "DiffuseColor" | "Diffuse" => {
                        let r = prop_f32(p, 4);
                        let g = prop_f32(p, 5);
                        let b = prop_f32(p, 6);
                        mat.base_color_factor = Vec4::new(r, g, b, 1.0);
                    }
                    "EmissiveColor" | "Emissive" => {
                        let r = prop_f32(p, 4);
                        let g = prop_f32(p, 5);
                        let b = prop_f32(p, 6);
                        mat.emissive_factor = Vec3::new(r, g, b);
                    }
                    _ => {}
                }
            }
        }

        let idx = self.mats.len();
        self.mat_fbx.insert(id, idx);
        self.mats.push(RawMat { fbx_id: id, material: mat });
    }

    fn extract_texture(&mut self, node: &FbxNode) {
        let id   = node.id().unwrap_or(0);
        let name = fbx_object_name(node);
        let uri  = node.child("FileName")
            .or_else(|| node.child("RelativeFilename"))
            .and_then(|n| n.as_str())
            .unwrap_or("").to_owned();

        let idx = self.texs.len();
        self.tex_fbx.insert(id, idx);
        self.texs.push(RawTex { fbx_id: id, image_uri: uri, name });
    }

    fn extract_model(&mut self, node: &FbxNode) {
        let id   = node.id().unwrap_or(0);
        let name = fbx_object_name(node);

        let mut translation = Vec3::ZERO;
        let mut rotation_deg = Vec3::ZERO;
        let mut scale       = Vec3::ONE;

        if let Some(props) = node.child("Properties70") {
            for p in props.children_named("P") {
                let pname = match p.properties.first().and_then(FbxProperty::as_str) {
                    Some(s) => s,
                    None    => continue,
                };
                match pname {
                    "LclTranslation" | "Lcl Translation" => {
                        translation = Vec3::new(prop_f32(p, 4), prop_f32(p, 5), prop_f32(p, 6));
                    }
                    "LclRotation" | "Lcl Rotation" => {
                        rotation_deg = Vec3::new(prop_f32(p, 4), prop_f32(p, 5), prop_f32(p, 6));
                    }
                    "LclScaling" | "Lcl Scaling" => {
                        scale = Vec3::new(
                            prop_f32_default(p, 4, 1.0),
                            prop_f32_default(p, 5, 1.0),
                            prop_f32_default(p, 6, 1.0),
                        );
                    }
                    _ => {}
                }
            }
        }

        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            rotation_deg.x.to_radians(),
            rotation_deg.y.to_radians(),
            rotation_deg.z.to_radians(),
        );

        let idx = self.models.len();
        self.model_fbx.insert(id, idx);
        self.models.push(RawModel { fbx_id: id, name, translation, rotation, scale });
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fbx_object_name(node: &FbxNode) -> String {
    node.object_name()
        .unwrap_or("")
        .split('\x00').next().unwrap_or(node.name.as_str())
        .to_owned()
}

fn extract_f64_layer(geo: &FbxNode, layer: &str, key: &str) -> Vec<f64> {
    geo.child(layer)
        .and_then(|l| l.child(key))
        .and_then(|n| n.as_f64_slice())
        .map(|s| s.to_vec())
        .unwrap_or_default()
}

fn extract_mapping_mode(geo: &FbxNode, layer: &str) -> MappingMode {
    geo.child(layer)
        .and_then(|l| l.child("MappingInformationType"))
        .and_then(|n| n.as_str())
        .map(MappingMode::from_str)
        .unwrap_or(MappingMode::ByPolygonVertex)
}

fn prop_f32(node: &FbxNode, idx: usize) -> f32 {
    node.properties.get(idx).and_then(FbxProperty::as_f64).unwrap_or(0.0) as f32
}

fn prop_f32_default(node: &FbxNode, idx: usize, default: f32) -> f32 {
    node.properties.get(idx).and_then(FbxProperty::as_f64).map(|v| v as f32).unwrap_or(default)
}

#[derive(Clone, Copy, PartialEq)]
enum MappingMode {
    ByPolygonVertex,
    ByVertex,
}

impl MappingMode {
    fn from_str(s: &str) -> Self {
        match s {
            "ByVertex" | "ByVertice" => MappingMode::ByVertex,
            _ => MappingMode::ByPolygonVertex,
        }
    }
}
