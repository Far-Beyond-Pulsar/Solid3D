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
                    let kv_str: Vec<String> = axis_vals.iter().map(|v| {
                        let s = format!("{v}");
                        if s.contains('.') || s.contains('e') || s.contains('E') { s } else { format!("{v}.0") }
                    }).collect();
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

        // Tangents
        let has_tangents = mesh.vertices.iter().any(|v| v.tangent.is_some());
        if has_tangents {
            let tangent_xyz: Vec<f64> = mesh.vertices.iter()
                .flat_map(|v| {
                    let t = v.tangent.unwrap_or(glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
                    [t.x as f64, t.y as f64, t.z as f64]
                }).collect();
            let tangent_w: Vec<f64> = mesh.vertices.iter()
                .map(|v| v.tangent.map_or(1.0, |t| t.w as f64))
                .collect();
            self.line("LayerElementTangent: 0 {")?;
            self.indent += 1;
            self.line("Version: 101")?;
            self.line("Name: \"\"")?;
            self.line("MappingInformationType: \"ByPolygonVertex\"")?;
            self.line("ReferenceInformationType: \"Direct\"")?;
            self.write_f64_array("Tangents", &tangent_xyz)?;
            self.write_f64_array("TangentW", &tangent_w)?;
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

        // Per-polygon material layer
        let has_multi_mat = mesh.primitives.len() > 1
            || mesh.primitives.first().and_then(|p| p.material_index).is_some();
        if has_multi_mat {
            // Compute triangle count per primitive
            let (mapping, mat_indices): (&str, Vec<i32>) = if mesh.primitives.len() <= 1 {
                ("AllSame", vec![mesh.primitives.first()
                    .and_then(|p| p.material_index)
                    .map(|_| 0_i32)
                    .unwrap_or(0)])
            } else {
                let mut poly_mats: Vec<i32> = Vec::new();
                for (local_idx, prim) in mesh.primitives.iter().enumerate() {
                    let tri_count = prim.indices.len() / 3;
                    for _ in 0..tri_count {
                        poly_mats.push(local_idx as i32);
                    }
                }
                ("ByPolygon", poly_mats)
            };
            self.line("LayerElementMaterial: 0 {")?;
            self.indent += 1;
            self.line(&format!("MappingInformationType: \"{mapping}\""))?;
            self.line("ReferenceInformationType: \"IndexToDirect\"")?;
            self.write_i32_array("Materials", &mat_indices)?;
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
        } else if let Projection::Orthographic(o) = &cam.projection {
            self.line("P: \"CameraProjectionType\", \"enum\", \"\", \"\",1")?;
            self.line(&format!(
                "P: \"OrthoZoom\", \"double\", \"Number\", \"\",{}", o.x_mag
            ))?;
            self.line(&format!(
                "P: \"NearPlane\", \"double\", \"Number\", \"\",{}", o.z_near
            ))?;
            self.line(&format!(
                "P: \"FarPlane\", \"double\", \"Number\", \"\",{}", o.z_far
            ))?;
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
            Light::Area(_)        => 3,
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

        if let Light::Area(a) = light {
            let area_size = a.width.max(a.height);
            self.line(&format!(
                "P: \"AreaSize\", \"double\", \"Number\", \"\",{area_size}"
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
        let items: Vec<String> = data.iter().map(|v| {
            let s = format!("{v}");
            // Always include a decimal point so the ASCII parser creates ArrFloat64
            if s.contains('.') || s.contains('e') || s.contains('E') { s } else { format!("{v}.0") }
        }).collect();
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

// ═══════════════════════════════════════════════════════════════════════════════
// Binary FBX 7.4 save
// ═══════════════════════════════════════════════════════════════════════════════

/// A node in the in-memory binary FBX tree.
struct BinNode {
    name:     String,
    props:    Vec<BinProp>,
    children: Vec<BinNode>,
}

/// A typed property value for the binary format.
enum BinProp {
    Int32(i32),
    Int64(i64),
    Float64(f64),
    Str(String),
    F64Arr(Vec<f64>),
    I32Arr(Vec<i32>),
}

impl BinNode {
    fn leaf(name: &str, props: Vec<BinProp>) -> Self {
        Self { name: name.to_string(), props, children: vec![] }
    }
}

/// Growable byte buffer with u32 back-patching.
struct BinBuf(Vec<u8>);

impl BinBuf {
    fn new() -> Self { Self(Vec::new()) }
    fn pos(&self) -> u32 { self.0.len() as u32 }
    fn push_u8(&mut self, v: u8) { self.0.push(v); }
    fn push_u32(&mut self, v: u32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn push_i32(&mut self, v: i32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn push_i64(&mut self, v: i64) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn push_f64(&mut self, v: f64) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn push_bytes(&mut self, v: &[u8]) { self.0.extend_from_slice(v); }
    fn patch_u32(&mut self, pos: usize, v: u32) {
        self.0[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
    }
}

fn bin_write_prop(buf: &mut BinBuf, prop: &BinProp) {
    match prop {
        BinProp::Int32(v)  => { buf.push_u8(b'I'); buf.push_i32(*v); }
        BinProp::Int64(v)  => { buf.push_u8(b'L'); buf.push_i64(*v); }
        BinProp::Float64(v) => { buf.push_u8(b'D'); buf.push_f64(*v); }
        BinProp::Str(s) => {
            buf.push_u8(b'S');
            let b = s.as_bytes();
            buf.push_u32(b.len() as u32);
            buf.push_bytes(b);
        }
        BinProp::F64Arr(arr) => {
            buf.push_u8(b'd');
            buf.push_u32(arr.len() as u32);
            buf.push_u32(0); // encoding = raw
            buf.push_u32((arr.len() * 8) as u32);
            for &v in arr { buf.push_f64(v); }
        }
        BinProp::I32Arr(arr) => {
            buf.push_u8(b'i');
            buf.push_u32(arr.len() as u32);
            buf.push_u32(0); // encoding = raw
            buf.push_u32((arr.len() * 4) as u32);
            for &v in arr { buf.push_i32(v); }
        }
    }
}

fn bin_write_node(buf: &mut BinBuf, node: &BinNode) {
    let end_offset_pos = buf.pos() as usize;
    buf.push_u32(0); // placeholder: end_offset (absolute)
    buf.push_u32(node.props.len() as u32);
    let prop_len_pos = buf.pos() as usize;
    buf.push_u32(0); // placeholder: property_list_len

    let name_bytes = node.name.as_bytes();
    buf.push_u8(name_bytes.len() as u8);
    buf.push_bytes(name_bytes);

    let props_start = buf.pos() as usize;
    for prop in &node.props { bin_write_prop(buf, prop); }
    let prop_list_len = (buf.pos() as usize - props_start) as u32;
    buf.patch_u32(prop_len_pos, prop_list_len);

    for child in &node.children { bin_write_node(buf, child); }

    if !node.children.is_empty() {
        buf.push_bytes(&[0u8; 13]); // null sentinel after child list
    }

    buf.patch_u32(end_offset_pos, buf.pos());
}

fn bin_build_geometry(id: i64, mesh: &solid_rs::scene::Mesh) -> BinNode {
    let obj_name = format!("{}\x00\x01Geometry", mesh.name);

    let verts: Vec<f64> = mesh.vertices.iter()
        .flat_map(|v| [v.position.x as f64, v.position.y as f64, v.position.z as f64])
        .collect();

    let mut pvi: Vec<i32> = Vec::new();
    for prim in &mesh.primitives {
        let idx = &prim.indices;
        let n   = idx.len();
        for (j, &vi) in idx.iter().enumerate() {
            if j == n - 1 { pvi.push(!(vi as i32)); } else { pvi.push(vi as i32); }
        }
    }

    let mut children = vec![
        BinNode::leaf("Vertices",            vec![BinProp::F64Arr(verts)]),
        BinNode::leaf("PolygonVertexIndex",  vec![BinProp::I32Arr(pvi)]),
    ];

    let normals: Vec<f64> = mesh.vertices.iter()
        .flat_map(|v| { let n = v.normal.unwrap_or(Vec3::Y); [n.x as f64, n.y as f64, n.z as f64] })
        .collect();
    if !normals.is_empty() {
        children.push(BinNode {
            name: "LayerElementNormal".to_string(),
            props: vec![BinProp::Int32(0)],
            children: vec![
                BinNode::leaf("MappingInformationType", vec![BinProp::Str("ByPolygonVertex".to_string())]),
                BinNode::leaf("ReferenceInformationType", vec![BinProp::Str("Direct".to_string())]),
                BinNode::leaf("Normals", vec![BinProp::F64Arr(normals)]),
            ],
        });
    }

    let uvs: Vec<f64> = mesh.vertices.iter()
        .flat_map(|v| { let uv = v.uvs[0].unwrap_or_default(); [uv.x as f64, (1.0 - uv.y) as f64] })
        .collect();
    if !uvs.is_empty() {
        children.push(BinNode {
            name: "LayerElementUV".to_string(),
            props: vec![BinProp::Int32(0)],
            children: vec![
                BinNode::leaf("MappingInformationType", vec![BinProp::Str("ByPolygonVertex".to_string())]),
                BinNode::leaf("ReferenceInformationType", vec![BinProp::Str("Direct".to_string())]),
                BinNode::leaf("UV", vec![BinProp::F64Arr(uvs)]),
            ],
        });
    }

    BinNode {
        name: "Geometry".to_string(),
        props: vec![
            BinProp::Int64(id),
            BinProp::Str(obj_name),
            BinProp::Str("Mesh".to_string()),
        ],
        children,
    }
}

fn bin_build_model(id: i64, node: &solid_rs::scene::Node) -> BinNode {
    let obj_name = format!("{}\x00\x01Model", node.name);
    let t = &node.transform;
    let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);

    BinNode {
        name: "Model".to_string(),
        props: vec![
            BinProp::Int64(id),
            BinProp::Str(obj_name),
            BinProp::Str("Mesh".to_string()),
        ],
        children: vec![BinNode {
            name: "Properties70".to_string(),
            props: vec![],
            children: vec![
                BinNode::leaf("P", vec![
                    BinProp::Str("Lcl Translation".to_string()),
                    BinProp::Str("Lcl Translation".to_string()),
                    BinProp::Str(String::new()),
                    BinProp::Str("A".to_string()),
                    BinProp::Float64(t.translation.x as f64),
                    BinProp::Float64(t.translation.y as f64),
                    BinProp::Float64(t.translation.z as f64),
                ]),
                BinNode::leaf("P", vec![
                    BinProp::Str("Lcl Rotation".to_string()),
                    BinProp::Str("Lcl Rotation".to_string()),
                    BinProp::Str(String::new()),
                    BinProp::Str("A".to_string()),
                    BinProp::Float64(rx.to_degrees() as f64),
                    BinProp::Float64(ry.to_degrees() as f64),
                    BinProp::Float64(rz.to_degrees() as f64),
                ]),
                BinNode::leaf("P", vec![
                    BinProp::Str("Lcl Scaling".to_string()),
                    BinProp::Str("Lcl Scaling".to_string()),
                    BinProp::Str(String::new()),
                    BinProp::Str("A".to_string()),
                    BinProp::Float64(t.scale.x as f64),
                    BinProp::Float64(t.scale.y as f64),
                    BinProp::Float64(t.scale.z as f64),
                ]),
            ],
        }],
    }
}

fn bin_build_material(id: i64, mat: &solid_rs::scene::Material) -> BinNode {
    let obj_name  = format!("{}\x00\x01Material", mat.name);
    let c         = mat.base_color_factor;
    let e         = mat.emissive_factor;
    let shininess = if mat.roughness_factor > 0.0 {
        (2.0_f64 / (mat.roughness_factor as f64).powi(2) - 2.0).max(0.0)
    } else {
        10000.0
    };

    BinNode {
        name: "Material".to_string(),
        props: vec![
            BinProp::Int64(id),
            BinProp::Str(obj_name),
            BinProp::Str(String::new()),
        ],
        children: vec![
            BinNode::leaf("ShadingModel", vec![BinProp::Str("phong".to_string())]),
            BinNode {
                name: "Properties70".to_string(),
                props: vec![],
                children: vec![
                    BinNode::leaf("P", vec![
                        BinProp::Str("DiffuseColor".to_string()),
                        BinProp::Str("Color".to_string()),
                        BinProp::Str(String::new()),
                        BinProp::Str("A".to_string()),
                        BinProp::Float64(c.x as f64),
                        BinProp::Float64(c.y as f64),
                        BinProp::Float64(c.z as f64),
                    ]),
                    BinNode::leaf("P", vec![
                        BinProp::Str("EmissiveColor".to_string()),
                        BinProp::Str("Color".to_string()),
                        BinProp::Str(String::new()),
                        BinProp::Str("A".to_string()),
                        BinProp::Float64(e.x as f64),
                        BinProp::Float64(e.y as f64),
                        BinProp::Float64(e.z as f64),
                    ]),
                    BinNode::leaf("P", vec![
                        BinProp::Str("Shininess".to_string()),
                        BinProp::Str("Number".to_string()),
                        BinProp::Str(String::new()),
                        BinProp::Str("A".to_string()),
                        BinProp::Float64(shininess),
                    ]),
                ],
            },
        ],
    }
}

fn bin_build_scene(scene: &Scene) -> Vec<BinNode> {
    let mut id_counter: i64 = 0;
    let mut next = || { id_counter += 1000; id_counter };

    let geom_ids:  Vec<i64> = (0..scene.meshes.len()).map(|_| next()).collect();
    let model_ids: Vec<i64> = (0..scene.nodes.len()).map(|_| next()).collect();
    let mat_ids:   Vec<i64> = (0..scene.materials.len()).map(|_| next()).collect();

    let total = scene.meshes.len() + scene.nodes.len() + scene.materials.len();

    let header_ext = BinNode {
        name: "FBXHeaderExtension".to_string(),
        props: vec![],
        children: vec![
            BinNode::leaf("FBXVersion", vec![BinProp::Int32(7400)]),
            BinNode::leaf("Creator",    vec![BinProp::Str("SolidRS".to_string())]),
        ],
    };

    let definitions = BinNode {
        name: "Definitions".to_string(),
        props: vec![],
        children: vec![
            BinNode::leaf("Count", vec![BinProp::Int32(total as i32)]),
            BinNode {
                name: "ObjectType".to_string(),
                props: vec![BinProp::Str("Model".to_string())],
                children: vec![BinNode::leaf("Count", vec![BinProp::Int32(scene.nodes.len() as i32)])],
            },
            BinNode {
                name: "ObjectType".to_string(),
                props: vec![BinProp::Str("Geometry".to_string())],
                children: vec![BinNode::leaf("Count", vec![BinProp::Int32(scene.meshes.len() as i32)])],
            },
            BinNode {
                name: "ObjectType".to_string(),
                props: vec![BinProp::Str("Material".to_string())],
                children: vec![BinNode::leaf("Count", vec![BinProp::Int32(scene.materials.len() as i32)])],
            },
        ],
    };

    let mut obj_children = Vec::new();
    for (i, mesh) in scene.meshes.iter().enumerate() {
        obj_children.push(bin_build_geometry(geom_ids[i], mesh));
    }
    for (i, node) in scene.nodes.iter().enumerate() {
        obj_children.push(bin_build_model(model_ids[i], node));
    }
    for (i, mat) in scene.materials.iter().enumerate() {
        obj_children.push(bin_build_material(mat_ids[i], mat));
    }
    let objects = BinNode { name: "Objects".to_string(), props: vec![], children: obj_children };

    let node_id_map: std::collections::HashMap<u32, usize> = scene.nodes.iter()
        .enumerate()
        .map(|(i, n)| (n.id.0, i))
        .collect();

    let mut conn_children = Vec::new();
    for (ni, node) in scene.nodes.iter().enumerate() {
        let nid = model_ids[ni];

        if let Some(mi) = node.mesh {
            if mi < geom_ids.len() {
                conn_children.push(BinNode::leaf("C", vec![
                    BinProp::Str("OO".to_string()),
                    BinProp::Int64(geom_ids[mi]),
                    BinProp::Int64(nid),
                ]));
            }
            if mi < scene.meshes.len() {
                let mut written: std::collections::HashSet<usize> = Default::default();
                for prim in &scene.meshes[mi].primitives {
                    if let Some(mat_idx) = prim.material_index {
                        if written.insert(mat_idx) && mat_idx < mat_ids.len() {
                            conn_children.push(BinNode::leaf("C", vec![
                                BinProp::Str("OO".to_string()),
                                BinProp::Int64(mat_ids[mat_idx]),
                                BinProp::Int64(nid),
                            ]));
                        }
                    }
                }
            }
        }

        let parent_id = node.parent
            .and_then(|pid| node_id_map.get(&pid.0))
            .map(|&vi| model_ids[vi])
            .unwrap_or(0);
        conn_children.push(BinNode::leaf("C", vec![
            BinProp::Str("OO".to_string()),
            BinProp::Int64(nid),
            BinProp::Int64(parent_id),
        ]));
    }
    let connections = BinNode { name: "Connections".to_string(), props: vec![], children: conn_children };

    vec![header_ext, definitions, objects, connections]
}

impl FbxSaver {
    /// Save `scene` as a binary FBX 7.4 file.
    pub fn save_binary(&self, scene: &Scene, writer: &mut dyn Write) -> Result<()> {
        const MAGIC: &[u8; 23] = b"Kaydara FBX Binary  \x00\x1a\x00";

        let mut buf = BinBuf::new();
        buf.push_bytes(MAGIC);
        buf.push_u32(7400);

        for node in &bin_build_scene(scene) {
            bin_write_node(&mut buf, node);
        }
        buf.push_bytes(&[0u8; 13]); // document-level null terminator

        writer.write_all(&buf.0).map_err(SolidError::Io)
    }
}
