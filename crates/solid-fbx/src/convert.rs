//! FBX document → `solid_rs::Scene` conversion.
//!
//! This module walks the FBX DOM and constructs a `Scene` using
//! `SceneBuilder`.  Supported FBX features:
//!
//! * Geometry nodes → triangulated `Mesh` with positions, normals, UVs,
//!   vertex colours, and per-polygon material assignment
//! * Model nodes → `Node` with transform extracted from `Properties70`
//! * Material nodes → `Material` with PBR properties
//! * Texture nodes → `Texture` + backing `Image`
//! * NodeAttribute nodes → `Camera` or `Light` attached via OO connections
//! * OO/OP connections wiring the object graph together

use std::collections::HashMap;

use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4};

use solid_rs::builder::SceneBuilder;
use solid_rs::extensions::Extensions;
use solid_rs::geometry::{Primitive, SkinWeights, Vertex};
use solid_rs::scene::{
    AlphaMode, Animation, AnimationChannel, AnimationTarget, Camera, Image, ImageSource,
    Interpolation, Light, Material, Mesh, NodeId, Skin, TextureRef, Texture,
};
use solid_rs::scene::camera::{Projection, PerspectiveCamera};
use solid_rs::scene::light::{LightBase, DirectionalLight, PointLight, SpotLight};
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

/// Extracted geometry — raw data to build a `Mesh` in pass 2.
struct RawGeom {
    fbx_id:              i64,
    name:                String,
    vertices:            Vec<Vertex>,
    tri_indices:         Vec<u32>,
    /// For each triangle (0-based), the polygon index it was produced from.
    tri_poly_map:        Vec<usize>,
    /// Per-polygon local material indices (empty ⟹ `AllSame`).
    poly_mat_indices:    Vec<i32>,
    mat_mapping_all_same: bool,
    orig_vert_indices:   Vec<usize>,
    num_unique_verts:    usize,
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

/// Extracted camera attribute.
struct RawCam {
    fbx_id: i64,
    camera: Camera,
}

/// Extracted light attribute.
struct RawLight {
    fbx_id: i64,
    light:  Light,
}

// ── Converter ─────────────────────────────────────────────────────────────────

struct Converter<'d> {
    doc: &'d FbxDocument,

    // Pass-1 intermediates
    geoms:  Vec<RawGeom>,
    mats:   Vec<RawMat>,
    texs:   Vec<RawTex>,
    models: Vec<RawModel>,
    cams:   Vec<RawCam>,
    lights: Vec<RawLight>,

    // Pass-1 fbxID → intermediate vec index
    geom_fbx:  HashMap<i64, usize>,
    mat_fbx:   HashMap<i64, usize>,
    tex_fbx:   HashMap<i64, usize>,
    model_fbx: HashMap<i64, usize>,
    cam_fbx:   HashMap<i64, usize>,
    light_fbx: HashMap<i64, usize>,

    // Pass-1b connections (src_id, dst_id[, property_name])
    oo_conns: Vec<(i64, i64)>,
    op_conns: Vec<(i64, i64, String)>,
}

impl<'d> Converter<'d> {
    fn new(doc: &'d FbxDocument) -> Self {
        Self {
            doc,
            geoms:  Vec::new(),
            mats:   Vec::new(),
            texs:   Vec::new(),
            models: Vec::new(),
            cams:   Vec::new(),
            lights: Vec::new(),
            geom_fbx:  HashMap::new(),
            mat_fbx:   HashMap::new(),
            tex_fbx:   HashMap::new(),
            model_fbx: HashMap::new(),
            cam_fbx:   HashMap::new(),
            light_fbx: HashMap::new(),
            oo_conns: Vec::new(),
            op_conns: Vec::new(),
        }
    }

    fn run(&mut self) -> Result<Scene> {
        // ── Pass 1: extract objects ───────────────────────────────────────────
        if let Some(objects) = self.doc.find("Objects") {
            for child in &objects.children {
                match child.name.as_str() {
                    "Geometry"      => self.extract_geometry(child)?,
                    "Material"      => self.extract_material(child),
                    "Texture"       => self.extract_texture(child),
                    "Model"         => self.extract_model(child),
                    "NodeAttribute" => self.extract_node_attribute(child),
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
        let mut tex_image_map: Vec<usize> = Vec::with_capacity(self.texs.len());
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

        // Apply OP connections (texture → material property)
        let mut mat_diffuse_tex: HashMap<usize, usize> = HashMap::new();
        let mut mat_normal_tex:  HashMap<usize, usize> = HashMap::new();
        for (src_id, dst_id, prop) in &self.op_conns {
            if let (Some(&ti), Some(&mi)) = (self.tex_fbx.get(src_id), self.mat_fbx.get(dst_id)) {
                match prop.as_str() {
                    "DiffuseColor" | "Diffuse" => { mat_diffuse_tex.insert(mi, ti); }
                    "NormalMap" | "Bump"       => { mat_normal_tex.insert(mi, ti); }
                    _ => {}
                }
            }
        }

        // Push materials
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

        // Push cameras
        let mut cam_scene_idxs: Vec<usize> = Vec::with_capacity(self.cams.len());
        for raw in &self.cams {
            cam_scene_idxs.push(b.push_camera(raw.camera.clone()));
        }

        // Push lights
        let mut light_scene_idxs: Vec<usize> = Vec::with_capacity(self.lights.len());
        for raw in &self.lights {
            light_scene_idxs.push(b.push_light(raw.light.clone()));
        }

        // Build OO connection maps
        let mut model_to_mats:   HashMap<i64, Vec<i64>> = HashMap::new();
        let mut model_to_geom:   HashMap<i64, i64>      = HashMap::new();
        let mut model_to_parent: HashMap<i64, i64>      = HashMap::new();
        let mut model_to_cam:    HashMap<i64, i64>       = HashMap::new();
        let mut model_to_light:  HashMap<i64, i64>       = HashMap::new();

        for &(src_id, dst_id) in &self.oo_conns {
            if self.geom_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_geom.insert(dst_id, src_id);
            } else if self.mat_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_mats.entry(dst_id).or_default().push(src_id);
            } else if self.model_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_parent.insert(src_id, dst_id);
            } else if self.cam_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_cam.insert(dst_id, src_id);
            } else if self.light_fbx.contains_key(&src_id) && self.model_fbx.contains_key(&dst_id) {
                model_to_light.insert(dst_id, src_id);
            }
        }

        // Push meshes with per-primitive material indices
        let geom_to_model: HashMap<i64, i64> = model_to_geom.iter()
            .map(|(&model_id, &geom_id)| (geom_id, model_id))
            .collect();

        let mut geom_scene_idxs: Vec<usize> = vec![usize::MAX; self.geoms.len()];
        for (ri, raw) in self.geoms.iter().enumerate() {
            let model_id = geom_to_model.get(&raw.fbx_id).copied();
            let model_mats: Vec<i64> = model_id
                .and_then(|mid| model_to_mats.get(&mid))
                .cloned()
                .unwrap_or_default();

            let mut mesh = Mesh::new(&raw.name);
            mesh.vertices = raw.vertices.clone();

            if raw.mat_mapping_all_same || raw.poly_mat_indices.is_empty() {
                // Single primitive with first material
                let scene_mat = model_mats.first()
                    .and_then(|fbx_mid| self.mat_fbx.get(fbx_mid))
                    .map(|&mat_ri| mat_scene_idxs[mat_ri]);
                mesh.primitives = vec![Primitive::triangles(raw.tri_indices.clone(), scene_mat)];
            } else {
                // ByPolygon: group triangles by local material index
                let local_to_scene: Vec<Option<usize>> = model_mats.iter()
                    .map(|fbx_mid| {
                        self.mat_fbx.get(fbx_mid).map(|&mat_ri| mat_scene_idxs[mat_ri])
                    })
                    .collect();

                let mut mat_to_tris: HashMap<i32, Vec<u32>> = HashMap::new();
                for (t, &poly_idx) in raw.tri_poly_map.iter().enumerate() {
                    let local = raw.poly_mat_indices.get(poly_idx).copied().unwrap_or(0);
                    let base  = t * 3;
                    let entry = mat_to_tris.entry(local).or_default();
                    entry.push(raw.tri_indices[base]);
                    entry.push(raw.tri_indices[base + 1]);
                    entry.push(raw.tri_indices[base + 2]);
                }

                let mut sorted: Vec<(i32, Vec<u32>)> = mat_to_tris.into_iter().collect();
                sorted.sort_by_key(|(k, _)| *k);
                for (local_mat, tris) in sorted {
                    let scene_mat = local_to_scene.get(local_mat as usize).and_then(|&v| v);
                    mesh.primitives.push(Primitive::triangles(tris, scene_mat));
                }
            }

            geom_scene_idxs[ri] = b.push_mesh(mesh);
        }

        // Build node creation order: topological sort (roots first)
        let model_fbx_ids: Vec<i64> = self.models.iter().map(|m| m.fbx_id).collect();
        let mut created_nodes: HashMap<i64, NodeId> = HashMap::new();

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
                if created_nodes.contains_key(&parent_fbx) {
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

                    // Attach camera
                    if let Some(&cam_fbx_id) = model_to_cam.get(&id) {
                        if let Some(&cam_raw_idx) = self.cam_fbx.get(&cam_fbx_id) {
                            b.attach_camera(node_id, cam_scene_idxs[cam_raw_idx]);
                        }
                    }

                    // Attach light
                    if let Some(&light_fbx_id) = model_to_light.get(&id) {
                        if let Some(&light_raw_idx) = self.light_fbx.get(&light_fbx_id) {
                            b.attach_light(node_id, light_scene_idxs[light_raw_idx]);
                        }
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
        let id   = node.id().unwrap_or(0);
        let name = fbx_object_name(node);

        let verts: Vec<f64> = node.child("Vertices")
            .and_then(|n| n.as_f64_slice()).map(|s| s.to_vec()).unwrap_or_default();
        let pvi: Vec<i32> = node.child("PolygonVertexIndex")
            .and_then(|n| n.as_i32_slice()).map(|s| s.to_vec()).unwrap_or_default();

        if verts.is_empty() || pvi.is_empty() { return Ok(()); }

        let normals   = extract_f64_layer(node, "LayerElementNormal", "Normals");
        let uvs       = extract_f64_layer(node, "LayerElementUV", "UV");
        let norm_mode = extract_mapping_mode(node, "LayerElementNormal");
        let uv_mode   = extract_mapping_mode(node, "LayerElementUV");

        let (colors_data, color_indices, color_mode, color_ref_mode) =
            extract_color_layer(node);

        let (poly_mat_indices, mat_mapping_all_same) = extract_material_layer(node);

        let mut vertices:     Vec<Vertex> = Vec::new();
        let mut tri_indices:  Vec<u32>    = Vec::new();
        let mut tri_poly_map: Vec<usize>  = Vec::new();
        let mut poly_start  = 0usize;
        let mut flat_idx    = 0usize;
        let mut poly_idx    = 0usize;

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

            // Resolve vertex colour
            let color = if !colors_data.is_empty() {
                let cs = match color_mode { MappingMode::ByVertex => vert_idx, _ => flat_idx };
                let ci = if color_ref_mode == RefMode::IndexToDirect {
                    color_indices.get(cs).copied().unwrap_or(cs as i32) as usize
                } else {
                    cs
                };
                let cr = colors_data.get(ci*4  ).copied().unwrap_or(1.0) as f32;
                let cg = colors_data.get(ci*4+1).copied().unwrap_or(1.0) as f32;
                let cb = colors_data.get(ci*4+2).copied().unwrap_or(1.0) as f32;
                let ca = colors_data.get(ci*4+3).copied().unwrap_or(1.0) as f32;
                Some(Vec4::new(cr, cg, cb, ca))
            } else {
                None
            };

            let mut vtx = Vertex::new(Vec3::new(px, py, pz))
                .with_normal(Vec3::new(nx, ny, nz))
                .with_uv(Vec2::new(u, 1.0 - v)); // flip V for OpenGL
            if let Some(c) = color {
                vtx.colors[0] = Some(c);
            }
            vertices.push(vtx);
            flat_idx += 1;

            if is_last {
                let poly_len = i - poly_start + 1;
                let n_tris   = poly_len.saturating_sub(2);
                for fi in 1..=n_tris {
                    tri_indices.push(poly_start as u32);
                    tri_indices.push((poly_start + fi) as u32);
                    tri_indices.push((poly_start + fi + 1) as u32);
                    tri_poly_map.push(poly_idx);
                }
                poly_start = i + 1;
                poly_idx  += 1;
            }
        }

        let idx = self.geoms.len();
        self.geom_fbx.insert(id, idx);
        self.geoms.push(RawGeom {
            fbx_id: id,
            name,
            vertices,
            tri_indices,
            tri_poly_map,
            poly_mat_indices,
            mat_mapping_all_same,
        });
        Ok(())
    }

    fn extract_material(&mut self, node: &FbxNode) {
        let id   = node.id().unwrap_or(0);
        let name = fbx_object_name(node);

        let mut mat = Material::new(&name);

        // Track emissive separately so factor × color order doesn't matter
        let mut emissive_color  = Vec3::ZERO;
        let mut emissive_scale  = 1.0_f32;

        if let Some(props) = node.child("Properties70") {
            for p in props.children_named("P") {
                let pname = match p.properties.first().and_then(FbxProperty::as_str) {
                    Some(s) => s,
                    None    => continue,
                };
                match pname {
                    "DiffuseColor" | "Diffuse" => {
                        let r = prop_f32(p, 4);
                        let g = prop_f32(p, 5);
                        let b = prop_f32(p, 6);
                        mat.base_color_factor = Vec4::new(r, g, b, mat.base_color_factor.w);
                    }
                    "EmissiveColor" | "Emissive" => {
                        emissive_color = Vec3::new(prop_f32(p, 4), prop_f32(p, 5), prop_f32(p, 6));
                    }
                    "EmissiveFactor" => {
                        emissive_scale = prop_f32(p, 4).max(0.0);
                    }
                    "Shininess" | "ShininessExponent" => {
                        // roughness ≈ sqrt(2 / (shininess + 2))
                        let s = prop_f32(p, 4).max(0.0);
                        mat.roughness_factor = ((2.0 / (s as f64 + 2.0)).sqrt() as f32)
                            .clamp(0.0, 1.0);
                    }
                    "ReflectionFactor" | "SpecularFactor" => {
                        mat.metallic_factor = prop_f32(p, 4).clamp(0.0, 1.0);
                    }
                    "TransparencyFactor" => {
                        // 0.0 = fully opaque, 1.0 = fully transparent
                        let alpha = (1.0 - prop_f32(p, 4)).clamp(0.0, 1.0);
                        if alpha < 1.0 {
                            mat.alpha_mode = AlphaMode::Blend;
                            mat.base_color_factor.w = alpha;
                        }
                    }
                    "Opacity" => {
                        // 1.0 = fully opaque
                        let alpha = prop_f32(p, 4).clamp(0.0, 1.0);
                        if alpha < 1.0 {
                            mat.alpha_mode = AlphaMode::Blend;
                            mat.base_color_factor.w = alpha;
                        }
                    }
                    _ => {}
                }
            }
        }

        mat.emissive_factor = emissive_color * emissive_scale;

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

        let mut translation  = Vec3::ZERO;
        let mut rotation_deg = Vec3::ZERO;
        let mut scale        = Vec3::ONE;

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

    fn extract_node_attribute(&mut self, node: &FbxNode) {
        let id        = node.id().unwrap_or(0);
        let attr_type = node.properties.get(2).and_then(FbxProperty::as_str).unwrap_or("");
        match attr_type {
            "Camera" => self.extract_camera(id, node),
            "Light"  => self.extract_light(id, node),
            _ => {}
        }
    }

    fn extract_camera(&mut self, id: i64, node: &FbxNode) {
        let name       = fbx_object_name(node);
        let mut fov_y  = 45.0_f32.to_radians();
        let mut z_near = 0.01_f32;
        let mut z_far: Option<f32> = None;

        if let Some(props) = node.child("Properties70") {
            for p in props.children_named("P") {
                let pname = match p.properties.first().and_then(FbxProperty::as_str) {
                    Some(s) => s, None => continue,
                };
                match pname {
                    "FieldOfView" | "FieldOfViewX" => {
                        fov_y = (prop_f32(p, 4) as f64).to_radians() as f32;
                    }
                    "NearPlane" => {
                        z_near = prop_f32(p, 4).max(0.0001);
                    }
                    "FarPlane" => {
                        let v = prop_f32(p, 4);
                        if v > 0.0 { z_far = Some(v); }
                    }
                    _ => {}
                }
            }
        }

        let cam = Camera {
            name,
            projection: Projection::Perspective(PerspectiveCamera {
                fov_y,
                aspect_ratio: None,
                z_near,
                z_far,
            }),
            extensions: Extensions::new(),
        };

        let idx = self.cams.len();
        self.cam_fbx.insert(id, idx);
        self.cams.push(RawCam { fbx_id: id, camera: cam });
    }

    fn extract_light(&mut self, id: i64, node: &FbxNode) {
        let name         = fbx_object_name(node);
        let mut light_type: i64 = 0;
        let mut color    = Vec3::ONE;
        let mut intensity = 1.0_f32;
        let mut range: Option<f32> = None;
        let mut inner_angle = 0.0_f32;
        let mut outer_angle = std::f32::consts::FRAC_PI_4;

        if let Some(props) = node.child("Properties70") {
            for p in props.children_named("P") {
                let pname = match p.properties.first().and_then(FbxProperty::as_str) {
                    Some(s) => s, None => continue,
                };
                match pname {
                    "LightType" => {
                        light_type = p.properties.get(4).and_then(FbxProperty::as_i64).unwrap_or(0);
                    }
                    "Color" => {
                        color = Vec3::new(prop_f32(p, 4), prop_f32(p, 5), prop_f32(p, 6));
                    }
                    "Intensity" => {
                        intensity = (prop_f32(p, 4) / 100.0).max(0.0);
                    }
                    "DecayRange" => {
                        let r = prop_f32(p, 4);
                        if r > 0.0 { range = Some(r); }
                    }
                    "InnerAngle" => {
                        inner_angle = prop_f32(p, 4).to_radians();
                    }
                    "OuterAngle" => {
                        outer_angle = prop_f32(p, 4).to_radians();
                    }
                    _ => {}
                }
            }
        }

        let base  = LightBase { name, color, intensity };
        let light = match light_type {
            1 => Light::Directional(DirectionalLight { base, extensions: Extensions::new() }),
            2 => Light::Spot(SpotLight {
                base,
                range,
                inner_cone_angle: inner_angle,
                outer_cone_angle: outer_angle,
                extensions: Extensions::new(),
            }),
            _ => Light::Point(PointLight { base, range, extensions: Extensions::new() }),
        };

        let idx = self.lights.len();
        self.light_fbx.insert(id, idx);
        self.lights.push(RawLight { fbx_id: id, light });
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

/// Returns `(colors_f64, color_indices, mapping_mode, ref_mode)`.
fn extract_color_layer(geo: &FbxNode)
    -> (Vec<f64>, Vec<i32>, MappingMode, RefMode)
{
    let layer = match geo.child("LayerElementColor") {
        Some(l) => l,
        None    => return (Vec::new(), Vec::new(), MappingMode::ByPolygonVertex, RefMode::Direct),
    };

    let colors = layer.child("Colors")
        .and_then(|n| n.as_f64_slice())
        .map(|s| s.to_vec())
        .unwrap_or_default();

    let color_indices = layer.child("ColorIndex")
        .and_then(|n| n.as_i32_slice())
        .map(|s| s.to_vec())
        .unwrap_or_default();

    let mapping = layer.child("MappingInformationType")
        .and_then(|n| n.as_str())
        .map(MappingMode::from_str)
        .unwrap_or(MappingMode::ByPolygonVertex);

    let ref_mode = layer.child("ReferenceInformationType")
        .and_then(|n| n.as_str())
        .map(RefMode::from_str)
        .unwrap_or(RefMode::Direct);

    (colors, color_indices, mapping, ref_mode)
}

/// Returns `(per_polygon_mat_indices, all_same_flag)`.
fn extract_material_layer(geo: &FbxNode) -> (Vec<i32>, bool) {
    let layer = match geo.child("LayerElementMaterial") {
        Some(l) => l,
        None    => return (Vec::new(), true),
    };

    let mapping = layer.child("MappingInformationType")
        .and_then(|n| n.as_str())
        .unwrap_or("AllSame");

    if mapping == "AllSame" {
        return (Vec::new(), true);
    }

    let indices = layer.child("Materials")
        .and_then(|n| n.as_i32_slice())
        .map(|s| s.to_vec())
        .unwrap_or_default();

    (indices, false)
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

#[derive(Clone, Copy, PartialEq)]
enum RefMode {
    Direct,
    IndexToDirect,
}

impl RefMode {
    fn from_str(s: &str) -> Self {
        match s {
            "IndexToDirect" | "Index" => RefMode::IndexToDirect,
            _ => RefMode::Direct,
        }
    }
}
