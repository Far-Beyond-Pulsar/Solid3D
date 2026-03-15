//! `FbxSaver` — saves a `solid_rs::Scene` as an ASCII FBX 7.4 file.
//!
//! ASCII FBX was chosen for the saver because it is human-readable and
//! requires no separate binary serialisation infrastructure.

use std::io::Write;

use glam::{EulerRot, Mat4, Quat, Vec3};

use solid_rs::prelude::*;
use solid_rs::scene::{AlphaMode, Animation, AnimationTarget, Camera, Light, Scene};
use solid_rs::scene::camera::Projection;
use solid_rs::{Result, SolidError};

use crate::FBX_FORMAT;

struct SkinEntry {
    skin_id:   i64,
    geom_id:   i64,
    node_idx:  usize,
    skin_idx:  usize,
    clusters:  Vec<ClusterEntry>,
}

struct ClusterEntry {
    cluster_id:     i64,
    joint_node_idx: usize,
}

struct AnimEntry {
    stack_id: i64,
    layer_id: i64,
    channels: Vec<ChannelEntry>,
}

struct ChannelEntry {
    curve_node_id: i64,
    cx_id:         i64,
    cy_id:         i64,
    cz_id:         i64,
    chan_idx:       usize,
    anim_idx:       usize,
}

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
        let cam_ids:   Vec<i64> = (0..scene.cameras.len()).map(|_| next_id(&mut id_counter)).collect();
        let light_ids: Vec<i64> = (0..scene.lights.len()).map(|_| next_id(&mut id_counter)).collect();

        // ── Skin entries ──────────────────────────────────────────────────────
        let mut skin_entries: Vec<SkinEntry> = Vec::new();
        for (ni, node) in scene.nodes.iter().enumerate() {
            if let (Some(skin_idx), Some(mesh_idx)) = (node.skin, node.mesh) {
                if skin_idx >= scene.skins.len() || mesh_idx >= scene.meshes.len() { continue; }
                let skin = &scene.skins[skin_idx];
                let skin_id = next_id(&mut id_counter);
                let geom_id = mesh_ids[mesh_idx];
                let mut clusters = Vec::new();
                for joint_node_id in &skin.joints {
                    if let Some(jni) = scene.nodes.iter().position(|n| n.id == *joint_node_id) {
                        let cluster_id = next_id(&mut id_counter);
                        clusters.push(ClusterEntry { cluster_id, joint_node_idx: jni });
                    }
                }
                skin_entries.push(SkinEntry { skin_id, geom_id, node_idx: ni, skin_idx, clusters });
            }
        }

        // ── Animation entries ─────────────────────────────────────────────────
        let mut anim_entries: Vec<AnimEntry> = Vec::new();
        for (ai, anim) in scene.animations.iter().enumerate() {
            let stack_id = next_id(&mut id_counter);
            let layer_id = next_id(&mut id_counter);
            let mut channels = Vec::new();
            for (ci, _chan) in anim.channels.iter().enumerate() {
                let curve_node_id = next_id(&mut id_counter);
                let cx_id = next_id(&mut id_counter);
                let cy_id = next_id(&mut id_counter);
                let cz_id = next_id(&mut id_counter);
                channels.push(ChannelEntry { curve_node_id, cx_id, cy_id, cz_id, chan_idx: ci, anim_idx: ai });
            }
            anim_entries.push(AnimEntry { stack_id, layer_id, channels });
        }

        // ── Definitions ──────────────────────────────────────────────────────
        let skin_obj_count: usize = skin_entries.iter().map(|e| 1 + e.clusters.len()).sum();
        let anim_obj_count: usize = anim_entries.iter().map(|e| 2 + e.channels.len() * 4).sum();
        let total = scene.meshes.len() + scene.materials.len()
                  + scene.textures.len() + scene.nodes.len()
                  + scene.cameras.len() + scene.lights.len()
                  + skin_obj_count + anim_obj_count;
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
            let node_type = if node.mesh.is_some() {
                "Mesh"
            } else if node.camera.is_some() {
                "Camera"
            } else if node.light.is_some() {
                "Light"
            } else {
                "Null"
            };
            self.write_model(node_ids[i], node, node_type)?;
        }
        for (i, mat) in scene.materials.iter().enumerate() {
            self.write_material(mat_ids[i], mat)?;
        }
        for (i, tex) in scene.textures.iter().enumerate() {
            let uri = scene.images
                .get(tex.image_index)
                .and_then(|img| if let solid_rs::scene::ImageSource::Uri(u) = &img.source { Some(u.as_str()) } else { None })
                .unwrap_or("");
            self.write_texture(tex_ids[i], &tex.name, uri)?;
        }
        for (i, cam) in scene.cameras.iter().enumerate() {
            self.write_camera_attribute(cam_ids[i], cam)?;
        }
        for (i, light) in scene.lights.iter().enumerate() {
            self.write_light_attribute(light_ids[i], light)?;
        }

        // Write skin deformers
        for entry in &skin_entries {
            let skin = &scene.skins[entry.skin_idx];
            self.line(&format!("Deformer: {}, \"{}\", \"Skin\"  {{", entry.skin_id, escape(&skin.name)))?;
            self.indent += 1;
            self.line("Version: 101")?;
            self.indent -= 1;
            self.line("}")?;
            self.blank()?;

            let mesh = &scene.meshes[scene.nodes[entry.node_idx].mesh.unwrap()];
            for (ji, cluster_entry) in entry.clusters.iter().enumerate() {
                let joint_name = &scene.nodes[cluster_entry.joint_node_idx].name;
                let ibp = skin.inverse_bind_matrices.get(ji).copied().unwrap_or(Mat4::IDENTITY);
                let tl = ibp.inverse();
                let tl_cols: Vec<f64> = tl.to_cols_array().iter().map(|&x| x as f64).collect();

                let mut indexes: Vec<i32> = Vec::new();
                let mut weights: Vec<f64> = Vec::new();
                for (vi, v) in mesh.vertices.iter().enumerate() {
                    if let Some(sw) = &v.skin_weights {
                        for k in 0..4 {
                            if sw.joints[k] as usize == ji && sw.weights[k] > 0.0 {
                                indexes.push(vi as i32);
                                weights.push(sw.weights[k] as f64);
                                break;
                            }
                        }
                    }
                }

                self.line(&format!("Deformer: {}, \"{}\", \"Cluster\"  {{", cluster_entry.cluster_id, escape(joint_name)))?;
                self.indent += 1;
                self.line("Version: 100")?;
                self.write_i32_array("Indexes", &indexes)?;
                self.write_f64_array("Weights", &weights)?;
                self.write_f64_array("Transform", &[1.0,0.0,0.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0])?;
                self.write_f64_array("TransformLink", &tl_cols)?;
                self.indent -= 1;
                self.line("}")?;
                self.blank()?;
            }
        }

        // Write animations
        const FBX_TIME_UNIT: f64 = 46186158000.0;
        for (ae_idx, ae) in anim_entries.iter().enumerate() {
            let anim = &scene.animations[ae_idx];

            self.line(&format!("AnimationStack: {}, \"{}\", \"\"  {{", ae.stack_id, escape(&anim.name)))?;
            self.indent += 1;
            self.line("Properties70:  {")?;
            self.indent += 1;
            self.line(&format!("P: \"LocalStop\", \"KTime\", \"Time\", \"\",{}", (anim.duration() as f64 * FBX_TIME_UNIT) as i64))?;
            self.indent -= 1;
            self.line("}")?;
            self.indent -= 1;
            self.line("}")?;
            self.blank()?;

            self.line(&format!("AnimationLayer: {}, \"BaseLayer\", \"\"  {{", ae.layer_id))?;
            self.line("}")?;
            self.blank()?;

            for ce in &ae.channels {
                let chan = &scene.animations[ce.anim_idx].channels[ce.chan_idx];
                let prop_name = match &chan.target {
                    AnimationTarget::Translation(_) => "T",
                    AnimationTarget::Rotation(_)    => "R",
                    AnimationTarget::Scale(_)        => "S",
                    _                                => "T",
                };

                self.line(&format!("AnimationCurveNode: {}, \"AnimCurveNode::{prop_name}\", \"\"  {{", ce.curve_node_id))?;
                self.indent += 1;
                self.line("Properties70:  {")?;
                self.indent += 1;
                self.line("P: \"d|X\", \"Number\", \"\", \"A\",0")?;
                self.line("P: \"d|Y\", \"Number\", \"\", \"A\",0")?;
                self.line("P: \"d|Z\", \"Number\", \"\", \"A\",0")?;
                self.indent -= 1;
                self.line("}")?;
                self.indent -= 1;
                self.line("}")?;
                self.blank()?;

                let (x_vals, y_vals, z_vals) = match &chan.target {
                    AnimationTarget::Translation(_) | AnimationTarget::Scale(_) => {
                        let x: Vec<f64> = chan.values.iter().step_by(3).map(|&v| v as f64).collect();
                        let y: Vec<f64> = chan.values.iter().skip(1).step_by(3).map(|&v| v as f64).collect();
                        let z: Vec<f64> = chan.values.iter().skip(2).step_by(3).map(|&v| v as f64).collect();
                        (x, y, z)
                    },
                    AnimationTarget::Rotation(_) => {
                        let mut x = Vec::new(); let mut y = Vec::new(); let mut z = Vec::new();
                        for i in 0..chan.times.len() {
                            let qx = chan.values.get(i*4).copied().unwrap_or(0.0);
                            let qy = chan.values.get(i*4+1).copied().unwrap_or(0.0);
                            let qz = chan.values.get(i*4+2).copied().unwrap_or(0.0);
                            let qw = chan.values.get(i*4+3).copied().unwrap_or(1.0);
                            let q = Quat::from_xyzw(qx, qy, qz, qw);
                            let (rx, ry, rz) = q.to_euler(EulerRot::XYZ);
                            x.push(rx.to_degrees() as f64);
                            y.push(ry.to_degrees() as f64);
                            z.push(rz.to_degrees() as f64);
                        }
                        (x, y, z)
                    },
                    _ => (Vec::new(), Vec::new(), Vec::new()),
                };

                let key_times_fbx: Vec<i64> = chan.times.iter().map(|&t| (t as f64 * FBX_TIME_UNIT) as i64).collect();

                for (axis_id, _axis_name, axis_vals) in [
                    (ce.cx_id, "X", &x_vals),
                    (ce.cy_id, "Y", &y_vals),
                    (ce.cz_id, "Z", &z_vals),
                ] {
                    self.line(&format!("AnimationCurve: {axis_id}, \"AnimCurve::\", \"\"  {{"))?;
                    self.indent += 1;
                    self.line("Default: 0")?;
                    self.line(&format!("KeyTime: *{} {{", key_times_fbx.len()))?;
                    self.indent += 1;
                    let kt_str: Vec<String> = key_times_fbx.iter().map(|v| v.to_string()).collect();
                    self.line(&format!("a: {}", kt_str.join(",")))?;
                    self.indent -= 1;
                    self.line("}")?;
                    let kv_str: Vec<String> = axis_vals.iter().map(|v| format!("{v}")).collect();
                    self.line(&format!("KeyValueFloat: *{} {{", axis_vals.len()))?;
                    self.indent += 1;
                    self.line(&format!("a: {}", kv_str.join(",")))?;
                    self.indent -= 1;
                    self.line("}")?;
                    self.indent -= 1;
                    self.line("}")?;
                    self.blank()?;
                }
            }
        }

        self.indent -= 1;
        self.line("}")?;
        self.blank()?;

        // ── Connections ───────────────────────────────────────────────────────
        self.line("Connections:  {")?;
        self.indent += 1;

        let node_id_to_vec: std::collections::HashMap<u32, usize> = scene.nodes
            .iter().enumerate().map(|(i, n)| (n.id.0, i)).collect();

        for (ni, node) in scene.nodes.iter().enumerate() {
            let nid = node_ids[ni];

            // Geometry → Model
            if let Some(mi) = node.mesh {
                self.line(&format!("C: \"OO\",{},{}", mesh_ids[mi], nid))?;
            }

            // All materials → Model (one connection per unique material)
            if let Some(mesh_idx) = node.mesh {
                let mut written: std::collections::HashSet<usize> = Default::default();
                for prim in &scene.meshes[mesh_idx].primitives {
                    if let Some(mat_idx) = prim.material_index {
                        if written.insert(mat_idx) && mat_idx < mat_ids.len() {
                            self.line(&format!("C: \"OO\",{},{}", mat_ids[mat_idx], nid))?;
                        }
                    }
                }
            }

            // Camera attribute → Model
            if let Some(ci) = node.camera {
                if ci < cam_ids.len() {
                    self.line(&format!("C: \"OO\",{},{}", cam_ids[ci], nid))?;
                }
            }

            // Light attribute → Model
            if let Some(li) = node.light {
                if li < light_ids.len() {
                    self.line(&format!("C: \"OO\",{},{}", light_ids[li], nid))?;
                }
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

        // Skin connections
        for entry in &skin_entries {
            self.line(&format!("C: \"OO\",{},{}", entry.skin_id, entry.geom_id))?;
            for cluster_entry in &entry.clusters {
                let cluster_id = cluster_entry.cluster_id;
                self.line(&format!("C: \"OO\",{},{}", cluster_id, entry.skin_id))?;
                let joint_nid = node_ids[cluster_entry.joint_node_idx];
                self.line(&format!("C: \"OO\",{},{}", joint_nid, cluster_id))?;
            }
        }

        // Animation connections
        for (ae_idx, ae) in anim_entries.iter().enumerate() {
            self.line(&format!("C: \"OO\",{},{}", ae.layer_id, ae.stack_id))?;
            self.line(&format!("C: \"OO\",{},0", ae.stack_id))?;
            for ce in &ae.channels {
                let chan = &scene.animations[ce.anim_idx].channels[ce.chan_idx];
                self.line(&format!("C: \"OO\",{},{}", ce.curve_node_id, ae.layer_id))?;
                let (target_node_id, prop_name_full) = match &chan.target {
                    AnimationTarget::Translation(nid) => (*nid, "Lcl Translation"),
                    AnimationTarget::Rotation(nid)    => (*nid, "Lcl Rotation"),
                    AnimationTarget::Scale(nid)        => (*nid, "Lcl Scaling"),
                    _ => continue,
                };
                if let Some(mi) = scene.nodes.iter().position(|n| n.id == target_node_id) {
                    self.line(&format!("C: \"OP\",{},{},\"{}\"", ce.curve_node_id, node_ids[mi], prop_name_full))?;
                }
                self.line(&format!("C: \"OP\",{},{},\"d|X\"", ce.cx_id, ce.curve_node_id))?;
                self.line(&format!("C: \"OP\",{},{},\"d|Y\"", ce.cy_id, ce.curve_node_id))?;
                self.line(&format!("C: \"OP\",{},{},\"d|Z\"", ce.cz_id, ce.curve_node_id))?;
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

        // PolygonVertexIndex from all primitives
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

        // Vertex colours
        let has_colors = mesh.vertices.iter().any(|v| v.colors[0].is_some());
        if has_colors {
            let color_data: Vec<f64> = mesh.vertices.iter()
                .flat_map(|v| {
                    let c = v.colors[0].unwrap_or(glam::Vec4::ONE);
                    [c.x as f64, c.y as f64, c.z as f64, c.w as f64]
                })
                .collect();
            self.line("LayerElementColor: 0 {")?;
            self.indent += 1;
            self.line("MappingInformationType: \"ByPolygonVertex\"")?;
            self.line("ReferenceInformationType: \"Direct\"")?;
            self.write_f64_array("Colors", &color_data)?;
            self.indent -= 1;
            self.line("}")?;
        }

        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    fn write_model(&mut self, id: i64, node: &solid_rs::scene::Node, node_type: &str) -> Result<()> {
        self.line(&format!(
            "Model: {id}, \"{}\", \"{}\"  {{", escape(&node.name), node_type
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
        // Always write EmissiveFactor = 1 since emissive_factor is already baked in
        self.line("P: \"EmissiveFactor\", \"Number\", \"\", \"A+\",1")?;

        // Shininess from roughness: shininess ≈ 2/r² − 2
        let shininess = if mat.roughness_factor > 0.0 {
            (2.0_f64 / (mat.roughness_factor as f64).powi(2) - 2.0).max(0.0)
        } else {
            10000.0
        };
        self.line(&format!(
            "P: \"Shininess\", \"Number\", \"\", \"A+\",{shininess}"
        ))?;

        self.line(&format!(
            "P: \"ReflectionFactor\", \"Number\", \"\", \"A+\",{}", mat.metallic_factor
        ))?;

        if mat.alpha_mode == AlphaMode::Blend {
            self.line(&format!(
                "P: \"Opacity\", \"Number\", \"\", \"A+\",{}", c.w
            ))?;
        }

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

    fn write_camera_attribute(&mut self, id: i64, cam: &Camera) -> Result<()> {
        self.line(&format!(
            "NodeAttribute: {id}, \"{}\", \"Camera\"  {{", escape(&cam.name)
        ))?;
        self.indent += 1;
        self.line("Properties70:  {")?;
        self.indent += 1;

        if let Projection::Perspective(p) = &cam.projection {
            let fov_deg = p.fov_y.to_degrees();
            self.line(&format!(
                "P: \"FieldOfView\", \"FieldOfView\", \"Number\", \"A+\",{fov_deg}"
            ))?;
            self.line(&format!(
                "P: \"NearPlane\", \"double\", \"Number\", \"\",{}", p.z_near
            ))?;
            if let Some(far) = p.z_far {
                self.line(&format!(
                    "P: \"FarPlane\", \"double\", \"Number\", \"\",{far}"
                ))?;
            }
        }

        self.indent -= 1;
        self.line("}")?;
        self.indent -= 1;
        self.line("}")?;
        self.blank()
    }

    fn write_light_attribute(&mut self, id: i64, light: &Light) -> Result<()> {
        self.line(&format!(
            "NodeAttribute: {id}, \"{}\", \"Light\"  {{", escape(light.name())
        ))?;
        self.indent += 1;
        self.line("Properties70:  {")?;
        self.indent += 1;

        let light_type: i32 = match light {
            Light::Point(_)       => 0,
            Light::Directional(_) => 1,
            Light::Spot(_)        => 2,
            Light::Area(_)        => 0, // no FBX area light; treat as point
        };
        self.line(&format!(
            "P: \"LightType\", \"enum\", \"\", \"\",{light_type}"
        ))?;

        let c = light.color();
        self.line(&format!(
            "P: \"Color\", \"Color\", \"\", \"A\",{},{},{}", c.x, c.y, c.z
        ))?;

        let intensity_100 = light.intensity() * 100.0;
        self.line(&format!(
            "P: \"Intensity\", \"Number\", \"\", \"A+\",{intensity_100}"
        ))?;

        if let Light::Spot(s) = light {
            self.line(&format!(
                "P: \"InnerAngle\", \"Number\", \"\", \"A+\",{}",
                s.inner_cone_angle.to_degrees()
            ))?;
            self.line(&format!(
                "P: \"OuterAngle\", \"Number\", \"\", \"A+\",{}",
                s.outer_cone_angle.to_degrees()
            ))?;
        }

        self.indent -= 1;
        self.line("}")?;
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
