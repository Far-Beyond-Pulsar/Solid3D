//! Bidirectional conversion between glTF DOM and solid-rs Scene.

use crate::buffer;
use crate::document::*;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use solid_rs::builder::SceneBuilder;
use solid_rs::geometry::{Primitive, SkinWeights, Transform, Vertex};
use solid_rs::scene::{
    AlphaMode, Animation, AnimationChannel, AnimationTarget, Camera,
    DirectionalLight, Image, ImageSource, Interpolation, Light, LightBase,
    Material, Mesh, MorphTarget, NodeId, OrthographicCamera, PerspectiveCamera,
    PointLight, Projection, Skin, SpotLight, Texture, TextureRef,
};
use solid_rs::{Result, SolidError};
use std::path::Path;

const DEFAULT_GLTF_SPECULAR_WEIGHT: f32 = 1.0;
const DEFAULT_GLTF_IOR: f32 = 1.5;
const GLTF_FLOAT_EPSILON: f32 = 1.0e-6;

pub fn gltf_to_scene(
    root: &GltfRoot,
    bin_chunk: &[u8],
    base_dir: Option<&Path>,
) -> Result<solid_rs::scene::scene::Scene> {
    let buffers = buffer::resolve_buffers(root, bin_chunk, base_dir)?;
    let mut b = if let Some(s) = root.scenes.first() {
        SceneBuilder::named(s.name.clone().unwrap_or_default())
    } else {
        SceneBuilder::new()
    };

    // --- Images ---
    for img in &root.images {
        let name = img.name.clone().unwrap_or_default();
        let solid_img = if let Some(uri) = &img.uri {
            if let Some(b64) = uri.strip_prefix("data:") {
                let comma = b64.find(',').ok_or_else(|| {
                    SolidError::parse("image data URI missing comma")
                })?;
                let mime = b64[..comma].split(';').next().unwrap_or("image/png");
                use base64::Engine;
                let data = base64::engine::general_purpose::STANDARD
                    .decode(&b64[comma + 1..])
                    .map_err(|e| SolidError::parse(format!("image base64: {e}")))?;
                Image::embedded(name, mime, data)
            } else {
                Image::from_uri(name, uri.clone())
            }
        } else if let Some(bv_idx) = img.buffer_view {
            let bv = &root.buffer_views[bv_idx];
            let buf = &buffers[bv.buffer];
            let data = buf[bv.byte_offset..bv.byte_offset + bv.byte_length].to_vec();
            let mime = img.mime_type.clone().unwrap_or_else(|| "image/png".into());
            Image::embedded(name, mime, data)
        } else {
            Image::from_uri(name, String::new())
        };
        b.push_image(solid_img);
    }

    // --- Textures ---
    for tex in &root.textures {
        let name = tex.name.clone().unwrap_or_default();
        let img_idx = tex.source.unwrap_or(0);
        b.push_texture(Texture::new(name, img_idx));
    }

    // --- Materials ---
    for mat in &root.materials {
        let mut m = Material::new(mat.name.clone().unwrap_or_default());
        if let Some(pbr) = &mat.pbr_metallic_roughness {
            if let Some(f) = pbr.base_color_factor {
                m.base_color_factor = Vec4::from(f);
            }
            m.metallic_factor  = pbr.metallic_factor.unwrap_or(1.0);
            m.roughness_factor = pbr.roughness_factor.unwrap_or(1.0);
            if let Some(ti) = &pbr.base_color_texture {
                m.base_color_texture = Some(TextureRef {
                    texture_index: ti.index,
                    uv_channel: ti.tex_coord.unwrap_or(0),
                    transform: None,
                });
            }
            if let Some(ti) = &pbr.metallic_roughness_texture {
                m.metallic_roughness_texture = Some(TextureRef {
                    texture_index: ti.index,
                    uv_channel: ti.tex_coord.unwrap_or(0),
                    transform: None,
                });
            }
        }
        if let Some(nt) = &mat.normal_texture {
            m.normal_texture = Some(TextureRef {
                texture_index: nt.index,
                uv_channel: nt.tex_coord.unwrap_or(0),
                transform: None,
            });
            m.normal_scale = nt.scale.unwrap_or(1.0);
        }
        if let Some(ot) = &mat.occlusion_texture {
            m.occlusion_texture = Some(TextureRef {
                texture_index: ot.index,
                uv_channel: ot.tex_coord.unwrap_or(0),
                transform: None,
            });
            m.occlusion_strength = ot.strength.unwrap_or(1.0);
        }
        if let Some(et) = &mat.emissive_texture {
            m.emissive_texture = Some(TextureRef {
                texture_index: et.index,
                uv_channel: et.tex_coord.unwrap_or(0),
                transform: None,
            });
        }
        if let Some(ef) = mat.emissive_factor {
            m.emissive_factor = Vec3::from(ef);
        }
        if let Some(ext) = &mat.extensions {
            // Preserve authored dielectric specular/IOR data instead of forcing
            // glTF's optional extensions to collapse into metallic-roughness.
            if let Some(specular) = &ext.khr_materials_specular {
                if let Some(color) = specular.specular_color_factor {
                    m.specular_color = Vec3::from(color);
                }
                if let Some(weight) = specular.specular_factor {
                    m.specular_weight = weight;
                }
            }
            if let Some(ior) = &ext.khr_materials_ior {
                if let Some(ior) = ior.ior {
                    m.ior = ior;
                }
            }
        }
        m.alpha_mode = match mat.alpha_mode.as_deref() {
            Some("MASK")  => AlphaMode::Mask,
            Some("BLEND") => AlphaMode::Blend,
            _             => AlphaMode::Opaque,
        };
        m.alpha_cutoff = mat.alpha_cutoff.unwrap_or(0.5);
        m.double_sided = mat.double_sided.unwrap_or(false);
        b.push_material(m);
    }

    // --- Meshes ---
    for gmesh in &root.meshes {
        let mut mesh = Mesh::new(gmesh.name.clone().unwrap_or_default());
        for prim in &gmesh.primitives {
            let pos_acc = prim.attributes.get("POSITION").copied();
            let positions: Vec<Vec3> = if let Some(idx) = pos_acc {
                let flat = buffer::read_f32(root, &buffers, idx)?;
                flat.chunks_exact(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect()
            } else {
                vec![]
            };

            let normals: Vec<Vec3> = if let Some(&idx) = prim.attributes.get("NORMAL") {
                let flat = buffer::read_f32(root, &buffers, idx)?;
                flat.chunks_exact(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect()
            } else {
                vec![]
            };

            let tangents: Vec<Vec4> = if let Some(&idx) = prim.attributes.get("TANGENT") {
                let flat = buffer::read_f32(root, &buffers, idx)?;
                flat.chunks_exact(4).map(|c| Vec4::new(c[0], c[1], c[2], c[3])).collect()
            } else {
                vec![]
            };

            let mut uv_channels: Vec<Vec<Vec2>> = Vec::new();
            for ch in 0..8usize {
                let key = format!("TEXCOORD_{ch}");
                if let Some(&idx) = prim.attributes.get(&key) {
                    let flat = buffer::read_f32(root, &buffers, idx)?;
                    uv_channels
                        .push(flat.chunks_exact(2).map(|c| Vec2::new(c[0], c[1])).collect());
                } else {
                    break;
                }
            }

            let colors: Vec<Vec4> = if let Some(&idx) = prim.attributes.get("COLOR_0") {
                let flat = buffer::read_f32(root, &buffers, idx)?;
                if flat.len() % 4 == 0 {
                    flat.chunks_exact(4).map(|c| Vec4::new(c[0], c[1], c[2], c[3])).collect()
                } else {
                    flat.chunks_exact(3).map(|c| Vec4::new(c[0], c[1], c[2], 1.0)).collect()
                }
            } else {
                vec![]
            };

            let joints_data: Vec<[u16; 4]> =
                if let Some(&idx) = prim.attributes.get("JOINTS_0") {
                    buffer::read_u16_vec4(root, &buffers, idx)?
                } else {
                    vec![]
                };

            let weights_data: Vec<[f32; 4]> =
                if let Some(&idx) = prim.attributes.get("WEIGHTS_0") {
                    let flat = buffer::read_f32(root, &buffers, idx)?;
                    flat.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect()
                } else {
                    vec![]
                };

            let n = positions.len();
            let vertices: Vec<Vertex> = (0..n)
                .map(|i| {
                    let mut v = Vertex::new(positions[i]);
                    if i < normals.len()  { v.normal  = Some(normals[i]); }
                    if i < tangents.len() { v.tangent = Some(tangents[i]); }
                    if i < colors.len()   { v.colors[0] = Some(colors[i]); }
                    for (ch, uvs) in uv_channels.iter().enumerate() {
                        if i < uvs.len() { v.uvs[ch] = Some(uvs[i]); }
                    }
                    if i < joints_data.len() && i < weights_data.len() {
                        v.skin_weights = Some(SkinWeights {
                            joints: joints_data[i],
                            weights: weights_data[i],
                        });
                    }
                    v
                })
                .collect();

            let indices: Vec<u32> = if let Some(idx) = prim.indices {
                buffer::read_u32(root, &buffers, idx)?
            } else {
                (0..n as u32).collect()
            };

            // Record vertex start offset before extending the shared buffer.
            let vert_offset = mesh.vertices.len();
            mesh.vertices.extend(vertices);
            let offset_indices: Vec<u32> =
                indices.iter().map(|&i| i + vert_offset as u32).collect();
            let solid_prim = Primitive::triangles(offset_indices, prim.material);
            mesh.primitives.push(solid_prim);
        }

        // --- Morph targets (second pass over primitives, concatenating per-prim deltas) ---
        let n_targets = gmesh.primitives.iter().map(|p| p.targets.len()).max().unwrap_or(0);
        if n_targets > 0 {
            let mut morph_targets: Vec<MorphTarget> = (0..n_targets)
                .map(|i| MorphTarget {
                    name: format!("target_{i}"),
                    ..Default::default()
                })
                .collect();
            for prim in &gmesh.primitives {
                for (ti, tgt_map) in prim.targets.iter().enumerate() {
                    if ti >= morph_targets.len() {
                        break;
                    }
                    if let Some(&pos_idx) = tgt_map.get("POSITION") {
                        let flat = buffer::read_f32(root, &buffers, pos_idx)?;
                        morph_targets[ti].position_deltas.extend(
                            flat.chunks_exact(3).map(|c| Vec3::new(c[0], c[1], c[2])),
                        );
                    }
                    if let Some(&nor_idx) = tgt_map.get("NORMAL") {
                        let flat = buffer::read_f32(root, &buffers, nor_idx)?;
                        morph_targets[ti].normal_deltas.extend(
                            flat.chunks_exact(3).map(|c| Vec3::new(c[0], c[1], c[2])),
                        );
                    }
                    if let Some(&tan_idx) = tgt_map.get("TANGENT") {
                        let flat = buffer::read_f32(root, &buffers, tan_idx)?;
                        morph_targets[ti].tangent_deltas.extend(
                            flat.chunks_exact(3).map(|c| Vec3::new(c[0], c[1], c[2])),
                        );
                    }
                }
            }
            mesh.morph_targets = morph_targets;
            mesh.morph_weights  = gmesh.weights.clone();
        }

        b.push_mesh(mesh);
    }

    // --- Cameras ---
    for gcam in &root.cameras {
        let projection = match gcam.type_.as_str() {
            "orthographic" => {
                let o = gcam.orthographic.as_ref().cloned().unwrap_or_default();
                Projection::Orthographic(OrthographicCamera {
                    x_mag: o.xmag,
                    y_mag: o.ymag,
                    z_near: o.znear,
                    z_far: o.zfar,
                })
            }
            _ => {
                let p = gcam.perspective.as_ref().cloned().unwrap_or_default();
                Projection::Perspective(PerspectiveCamera {
                    fov_y: p.yfov,
                    aspect_ratio: p.aspect_ratio,
                    z_near: p.znear,
                    z_far: p.zfar,
                })
            }
        };
        b.push_camera(Camera {
            name: gcam.name.clone().unwrap_or_default(),
            projection,
            extensions: solid_rs::extensions::Extensions::new(),
        });
    }

    // --- Nodes: BFS from scene roots, create hierarchy ---
    let root_node_indices: Vec<usize> = if let Some(si) = root.scene {
        if si < root.scenes.len() {
            root.scenes[si].nodes.clone()
        } else {
            (0..root.nodes.len()).collect()
        }
    } else if !root.scenes.is_empty() {
        root.scenes[0].nodes.clone()
    } else {
        (0..root.nodes.len()).collect()
    };

    let mut queue: std::collections::VecDeque<(usize, Option<NodeId>)> =
        root_node_indices.iter().map(|&i| (i, None)).collect();

    let mut gltf_to_node_map: std::collections::HashMap<usize, NodeId> =
        std::collections::HashMap::new();

    while let Some((gi, parent)) = queue.pop_front() {
        if gi >= root.nodes.len() {
            continue;
        }
        let gn = &root.nodes[gi];
        let name = gn.name.clone().unwrap_or_else(|| format!("Node_{gi}"));
        let node_id = if let Some(par) = parent {
            b.add_child_node(par, name)
        } else {
            b.add_root_node(name)
        };
        gltf_to_node_map.insert(gi, node_id);
        b.set_transform(node_id, node_transform(gn));
        if let Some(mi) = gn.mesh   { b.attach_mesh(node_id, mi); }
        if let Some(ci) = gn.camera { b.attach_camera(node_id, ci); }
        for &child in &gn.children {
            queue.push_back((child, Some(node_id)));
        }
    }

    // --- Skins ---
    for gskin in &root.skins {
        let mut skin = Skin::new(gskin.name.clone().unwrap_or_default());
        skin.skeleton_root = gskin.skeleton.and_then(|si| gltf_to_node_map.get(&si).copied());
        for &ji in &gskin.joints {
            if let Some(&nid) = gltf_to_node_map.get(&ji) {
                skin.joints.push(nid);
            }
        }
        if let Some(ibm_idx) = gskin.inverse_bind_matrices {
            let flat = buffer::read_f32(root, &buffers, ibm_idx)?;
            skin.inverse_bind_matrices = flat
                .chunks_exact(16)
                .map(|c| {
                    let mut arr = [0f32; 16];
                    arr.copy_from_slice(c);
                    Mat4::from_cols_array(&arr)
                })
                .collect();
        }
        b.push_skin(skin);
    }
    // Attach skins to the nodes that reference them.
    for (gi, gn) in root.nodes.iter().enumerate() {
        if let Some(skin_idx) = gn.skin {
            if let Some(&nid) = gltf_to_node_map.get(&gi) {
                b.attach_skin(nid, skin_idx);
            }
        }
    }

    // --- Animations ---
    for ganim in &root.animations {
        let mut anim = Animation::new(ganim.name.clone().unwrap_or_default());
        for gch in &ganim.channels {
            if gch.sampler >= ganim.samplers.len() {
                continue;
            }
            let sampler = &ganim.samplers[gch.sampler];
            let times  = buffer::read_f32(root, &buffers, sampler.input)?;
            let values = buffer::read_f32(root, &buffers, sampler.output)?;
            let interpolation = match sampler.interpolation.as_deref() {
                Some("STEP")        => Interpolation::Step,
                Some("CUBICSPLINE") => Interpolation::CubicSpline,
                _                   => Interpolation::Linear,
            };
            let target_gi = match gch.target.node {
                Some(ni) => ni,
                None => continue,
            };
            let node_id = match gltf_to_node_map.get(&target_gi) {
                Some(&nid) => nid,
                None => continue,
            };
            let target = match gch.target.path.as_str() {
                "translation" => AnimationTarget::Translation(node_id),
                "rotation"    => AnimationTarget::Rotation(node_id),
                "scale"       => AnimationTarget::Scale(node_id),
                "weights"     => AnimationTarget::MorphWeight { node_id, target_index: 0 },
                _ => continue,
            };
            anim.channels.push(AnimationChannel { target, interpolation, times, values });
        }
        b.push_animation(anim);
    }

    // --- KHR_lights_punctual (load) ---
    if let Some(root_ext) = &root.extensions {
        if let Some(khr) = root_ext.get("KHR_lights_punctual") {
            if let Some(lights) = khr.get("lights").and_then(|v| v.as_array()) {
                for light_val in lights {
                    b.push_light(parse_khr_light(light_val));
                }
            }
        }
    }
    for (gi, gn) in root.nodes.iter().enumerate() {
        if let Some(ext) = &gn.extensions {
            if let Some(khr) = ext.get("KHR_lights_punctual") {
                if let Some(light_idx) = khr.get("light").and_then(|v| v.as_u64()) {
                    if let Some(&nid) = gltf_to_node_map.get(&gi) {
                        b.attach_light(nid, light_idx as usize);
                    }
                }
            }
        }
    }

    Ok(b.build())
}

fn parse_khr_light(v: &serde_json::Value) -> Light {
    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
    let color = v
        .get("color")
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            if arr.len() >= 3 {
                Some(Vec3::new(
                    arr[0].as_f64().unwrap_or(1.0) as f32,
                    arr[1].as_f64().unwrap_or(1.0) as f32,
                    arr[2].as_f64().unwrap_or(1.0) as f32,
                ))
            } else {
                None
            }
        })
        .unwrap_or(Vec3::ONE);
    let intensity = v.get("intensity").and_then(|i| i.as_f64()).unwrap_or(1.0) as f32;
    let range     = v.get("range").and_then(|r| r.as_f64()).map(|r| r as f32);
    let base      = LightBase { name, color, intensity };
    let ext       = solid_rs::extensions::Extensions::new();

    match v.get("type").and_then(|t| t.as_str()).unwrap_or("point") {
        "directional" => Light::Directional(DirectionalLight { base, extensions: ext }),
        "spot" => {
            let inner = v
                .get("spot").and_then(|s| s.get("innerConeAngle"))
                .and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
            let outer = v
                .get("spot").and_then(|s| s.get("outerConeAngle"))
                .and_then(|a| a.as_f64())
                .unwrap_or(std::f64::consts::FRAC_PI_4) as f32;
            Light::Spot(SpotLight {
                base,
                range,
                inner_cone_angle: inner,
                outer_cone_angle: outer,
                extensions: ext,
            })
        }
        _ => Light::Point(PointLight { base, range, extensions: ext }),
    }
}

fn node_transform(gn: &GltfNode) -> Transform {
    if let Some(m) = gn.matrix {
        Transform::from_matrix(Mat4::from_cols_array(&m))
    } else {
        Transform {
            translation: gn.translation.map(Vec3::from).unwrap_or(Vec3::ZERO),
            rotation: gn.rotation
                .map(|r| Quat::from_xyzw(r[0], r[1], r[2], r[3]))
                .unwrap_or(Quat::IDENTITY),
            scale: gn.scale.map(Vec3::from).unwrap_or(Vec3::ONE),
        }
    }
}

pub fn scene_to_gltf(
    scene: &solid_rs::scene::scene::Scene,
) -> Result<(GltfRoot, Vec<u8>)> {
    let mut gltf = GltfRoot::default();
    gltf.asset = GltfAsset {
        version: "2.0".into(),
        generator: Some("solid-gltf 0.1.0 (SolidRS)".into()),
        min_version: None,
        copyright: None,
    };

    let mut bin: Vec<u8> = Vec::new();

    // --- Images ---
    for img in &scene.images {
        let gi = match &img.source {
            ImageSource::Uri(uri) => GltfImage {
                name: Some(img.name.clone()),
                uri: Some(uri.clone()),
                ..Default::default()
            },
            ImageSource::Embedded { mime_type, data } => {
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(data);
                let uri = format!("data:{mime_type};base64,{b64}");
                GltfImage {
                    name: Some(img.name.clone()),
                    uri: Some(uri),
                    mime_type: Some(mime_type.clone()),
                    ..Default::default()
                }
            }
        };
        gltf.images.push(gi);
    }

    // --- Textures ---
    for tex in &scene.textures {
        gltf.textures.push(GltfTexture {
            name: Some(tex.name.clone()),
            source: Some(tex.image_index),
            sampler: None,
        });
    }

    // --- Materials ---
    if scene.materials.iter().any(material_uses_specular_extension) {
        gltf.extensions_used.push("KHR_materials_specular".into());
    }
    if scene.materials.iter().any(material_uses_ior_extension) {
        gltf.extensions_used.push("KHR_materials_ior".into());
    }

    for mat in &scene.materials {
        let base_color_texture = mat.base_color_texture.as_ref().map(|tr| GltfTextureInfo {
            index: tr.texture_index,
            tex_coord: if tr.uv_channel > 0 { Some(tr.uv_channel) } else { None },
        });
        let mr_texture = mat.metallic_roughness_texture.as_ref().map(|tr| GltfTextureInfo {
            index: tr.texture_index,
            tex_coord: if tr.uv_channel > 0 { Some(tr.uv_channel) } else { None },
        });
        let normal_texture = mat.normal_texture.as_ref().map(|tr| GltfNormalTextureInfo {
            index: tr.texture_index,
            tex_coord: if tr.uv_channel > 0 { Some(tr.uv_channel) } else { None },
            scale: if (mat.normal_scale - 1.0).abs() > 1e-6 { Some(mat.normal_scale) } else { None },
        });
        let occlusion_texture = mat.occlusion_texture.as_ref().map(|tr| GltfOcclusionTextureInfo {
            index: tr.texture_index,
            tex_coord: if tr.uv_channel > 0 { Some(tr.uv_channel) } else { None },
            strength: if (mat.occlusion_strength - 1.0).abs() > 1e-6 { Some(mat.occlusion_strength) } else { None },
        });
        let emissive_texture = mat.emissive_texture.as_ref().map(|tr| GltfTextureInfo {
            index: tr.texture_index,
            tex_coord: if tr.uv_channel > 0 { Some(tr.uv_channel) } else { None },
        });
        let ef = mat.emissive_factor;
        let alpha_mode = match mat.alpha_mode {
            AlphaMode::Opaque => None,
            AlphaMode::Mask   => Some("MASK".into()),
            AlphaMode::Blend  => Some("BLEND".into()),
        };
        let f = mat.base_color_factor;
        let material_extensions = build_material_extensions(mat);
        gltf.materials.push(GltfMaterial {
            name: if mat.name.is_empty() { None } else { Some(mat.name.clone()) },
            pbr_metallic_roughness: Some(GltfPbr {
                base_color_factor: if f != glam::Vec4::ONE {
                    Some([f.x, f.y, f.z, f.w])
                } else {
                    None
                },
                base_color_texture,
                metallic_factor: if (mat.metallic_factor - 1.0).abs() > 1e-6 {
                    Some(mat.metallic_factor)
                } else {
                    None
                },
                roughness_factor: if (mat.roughness_factor - 1.0).abs() > 1e-6 {
                    Some(mat.roughness_factor)
                } else {
                    None
                },
                metallic_roughness_texture: mr_texture,
            }),
            normal_texture,
            occlusion_texture,
            emissive_texture,
            emissive_factor: if ef != glam::Vec3::ZERO { Some([ef.x, ef.y, ef.z]) } else { None },
            alpha_mode,
            alpha_cutoff: if mat.alpha_mode == AlphaMode::Mask { Some(mat.alpha_cutoff) } else { None },
            double_sided: if mat.double_sided { Some(true) } else { None },
            extensions: material_extensions,
        });
    }

    // --- Meshes: write vertex + index data into the binary buffer ---
    for mesh in &scene.meshes {
        let mut gprims: Vec<GltfPrimitive> = Vec::new();

        for prim in &mesh.primitives {
            let mut attributes = std::collections::HashMap::new();

            let n = mesh.vertices.len();
            let has_normals  = mesh.vertices.iter().any(|v| v.normal.is_some());
            let has_tangents = mesh.vertices.iter().any(|v| v.tangent.is_some());
            let has_uv0      = mesh.vertices.iter().any(|v| v.uvs[0].is_some());
            let has_color0   = mesh.vertices.iter().any(|v| v.colors[0].is_some());

            // POSITION
            let pos_offset = bin.len();
            for v in &mesh.vertices {
                let p = v.position;
                bin.extend_from_slice(&p.x.to_le_bytes());
                bin.extend_from_slice(&p.y.to_le_bytes());
                bin.extend_from_slice(&p.z.to_le_bytes());
            }
            let pos_bv = push_bv(&mut gltf, 0, pos_offset, n * 12, None, Some(34962));
            let pos_acc = push_acc(&mut gltf, pos_bv, 5126, n, "VEC3", 0);
            {
                let acc = gltf.accessors.last_mut().unwrap();
                let (mut min_v, mut max_v) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for v in &mesh.vertices {
                    min_v = min_v.min(v.position);
                    max_v = max_v.max(v.position);
                }
                acc.min = vec![min_v.x as f64, min_v.y as f64, min_v.z as f64];
                acc.max = vec![max_v.x as f64, max_v.y as f64, max_v.z as f64];
            }
            attributes.insert("POSITION".into(), pos_acc);

            // NORMAL
            if has_normals {
                let off = bin.len();
                for v in &mesh.vertices {
                    let nv = v.normal.unwrap_or(Vec3::Y);
                    bin.extend_from_slice(&nv.x.to_le_bytes());
                    bin.extend_from_slice(&nv.y.to_le_bytes());
                    bin.extend_from_slice(&nv.z.to_le_bytes());
                }
                let bv  = push_bv(&mut gltf, 0, off, n * 12, None, Some(34962));
                let acc = push_acc(&mut gltf, bv, 5126, n, "VEC3", 0);
                attributes.insert("NORMAL".into(), acc);
            }

            // TANGENT
            if has_tangents {
                let off = bin.len();
                for v in &mesh.vertices {
                    let t = v.tangent.unwrap_or(Vec4::new(1.0, 0.0, 0.0, 1.0));
                    bin.extend_from_slice(&t.x.to_le_bytes());
                    bin.extend_from_slice(&t.y.to_le_bytes());
                    bin.extend_from_slice(&t.z.to_le_bytes());
                    bin.extend_from_slice(&t.w.to_le_bytes());
                }
                let bv  = push_bv(&mut gltf, 0, off, n * 16, None, Some(34962));
                let acc = push_acc(&mut gltf, bv, 5126, n, "VEC4", 0);
                attributes.insert("TANGENT".into(), acc);
            }

            // TEXCOORD_0
            if has_uv0 {
                let off = bin.len();
                for v in &mesh.vertices {
                    let uv = v.uvs[0].unwrap_or(Vec2::ZERO);
                    bin.extend_from_slice(&uv.x.to_le_bytes());
                    bin.extend_from_slice(&uv.y.to_le_bytes());
                }
                let bv  = push_bv(&mut gltf, 0, off, n * 8, None, Some(34962));
                let acc = push_acc(&mut gltf, bv, 5126, n, "VEC2", 0);
                attributes.insert("TEXCOORD_0".into(), acc);
            }

            // COLOR_0
            if has_color0 {
                let off = bin.len();
                for v in &mesh.vertices {
                    let c = v.colors[0].unwrap_or(Vec4::ONE);
                    bin.extend_from_slice(&c.x.to_le_bytes());
                    bin.extend_from_slice(&c.y.to_le_bytes());
                    bin.extend_from_slice(&c.z.to_le_bytes());
                    bin.extend_from_slice(&c.w.to_le_bytes());
                }
                let bv  = push_bv(&mut gltf, 0, off, n * 16, None, Some(34962));
                let acc = push_acc(&mut gltf, bv, 5126, n, "VEC4", 0);
                attributes.insert("COLOR_0".into(), acc);
            }

            // JOINTS_0 + WEIGHTS_0
            let has_skin = mesh.vertices.iter().any(|v| v.skin_weights.is_some());
            if has_skin {
                // JOINTS_0: VEC4 UNSIGNED_SHORT
                let off = bin.len();
                for v in &mesh.vertices {
                    let j = v.skin_weights.as_ref().map(|s| s.joints).unwrap_or([0; 4]);
                    for ji in j {
                        bin.extend_from_slice(&ji.to_le_bytes());
                    }
                }
                let bv  = push_bv(&mut gltf, 0, off, n * 8, None, Some(34962));
                let acc = push_acc(&mut gltf, bv, 5123, n, "VEC4", 0);
                attributes.insert("JOINTS_0".into(), acc);

                // WEIGHTS_0: VEC4 FLOAT
                let off = bin.len();
                for v in &mesh.vertices {
                    let w = v.skin_weights.as_ref().map(|s| s.weights).unwrap_or([0.0; 4]);
                    for wi in w {
                        bin.extend_from_slice(&wi.to_le_bytes());
                    }
                }
                let bv  = push_bv(&mut gltf, 0, off, n * 16, None, Some(34962));
                let acc = push_acc(&mut gltf, bv, 5126, n, "VEC4", 0);
                attributes.insert("WEIGHTS_0".into(), acc);
            }

            // Morph targets — write delta accessors for each target, once per primitive
            // (mirrors how POSITION/NORMAL are written: all mesh vertices per primitive).
            let mut targets_vec: Vec<std::collections::HashMap<String, usize>> = Vec::new();
            for target in &mesh.morph_targets {
                let mut tgt_map: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();

                // POSITION deltas (required accessor; spec mandates min/max)
                if !target.position_deltas.is_empty() {
                    let off   = bin.len();
                    let count = n.min(target.position_deltas.len());
                    for d in target.position_deltas.iter().take(count) {
                        bin.extend_from_slice(&d.x.to_le_bytes());
                        bin.extend_from_slice(&d.y.to_le_bytes());
                        bin.extend_from_slice(&d.z.to_le_bytes());
                    }
                    for _ in count..n {
                        bin.extend_from_slice(&[0u8; 12]);
                    }
                    let bv    = push_bv(&mut gltf, 0, off, n * 12, None, None);
                    let acc_i = push_acc(&mut gltf, bv, 5126, n, "VEC3", 0);
                    {
                        let acc = gltf.accessors.last_mut().unwrap();
                        let (mut mn, mut mx) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                        for d in target.position_deltas.iter().take(count) {
                            mn = mn.min(*d);
                            mx = mx.max(*d);
                        }
                        if count == 0 {
                            mn = Vec3::ZERO;
                            mx = Vec3::ZERO;
                        }
                        acc.min = vec![mn.x as f64, mn.y as f64, mn.z as f64];
                        acc.max = vec![mx.x as f64, mx.y as f64, mx.z as f64];
                    }
                    tgt_map.insert("POSITION".into(), acc_i);
                }

                // NORMAL deltas
                if !target.normal_deltas.is_empty() {
                    let off   = bin.len();
                    let count = n.min(target.normal_deltas.len());
                    for d in target.normal_deltas.iter().take(count) {
                        bin.extend_from_slice(&d.x.to_le_bytes());
                        bin.extend_from_slice(&d.y.to_le_bytes());
                        bin.extend_from_slice(&d.z.to_le_bytes());
                    }
                    for _ in count..n {
                        bin.extend_from_slice(&[0u8; 12]);
                    }
                    let bv    = push_bv(&mut gltf, 0, off, n * 12, None, None);
                    let acc_i = push_acc(&mut gltf, bv, 5126, n, "VEC3", 0);
                    tgt_map.insert("NORMAL".into(), acc_i);
                }

                // TANGENT deltas (xyz only — morph tangents have no w)
                if !target.tangent_deltas.is_empty() {
                    let off   = bin.len();
                    let count = n.min(target.tangent_deltas.len());
                    for d in target.tangent_deltas.iter().take(count) {
                        bin.extend_from_slice(&d.x.to_le_bytes());
                        bin.extend_from_slice(&d.y.to_le_bytes());
                        bin.extend_from_slice(&d.z.to_le_bytes());
                    }
                    for _ in count..n {
                        bin.extend_from_slice(&[0u8; 12]);
                    }
                    let bv    = push_bv(&mut gltf, 0, off, n * 12, None, None);
                    let acc_i = push_acc(&mut gltf, bv, 5126, n, "VEC3", 0);
                    tgt_map.insert("TANGENT".into(), acc_i);
                }

                targets_vec.push(tgt_map);
            }

            // Indices (u32)
            let idx_off = bin.len();
            for &i in &prim.indices {
                bin.extend_from_slice(&i.to_le_bytes());
            }
            let idx_bv  = push_bv(&mut gltf, 0, idx_off, prim.indices.len() * 4, None, Some(34963));
            let idx_acc = push_acc(&mut gltf, idx_bv, 5125, prim.indices.len(), "SCALAR", 0);

            gprims.push(GltfPrimitive {
                attributes,
                indices: Some(idx_acc),
                material: prim.material_index,
                mode: None,
                targets: targets_vec,
            });
        }

        gltf.meshes.push(GltfMesh {
            name: if mesh.name.is_empty() { None } else { Some(mesh.name.clone()) },
            primitives: gprims,
            weights: mesh.morph_weights.clone(),
        });
    }

    // --- Cameras ---
    for cam in &scene.cameras {
        let (type_, perspective, orthographic) = match &cam.projection {
            Projection::Perspective(p) => (
                "perspective".into(),
                Some(GltfPerspective {
                    yfov: p.fov_y,
                    znear: p.z_near,
                    zfar: p.z_far,
                    aspect_ratio: p.aspect_ratio,
                }),
                None,
            ),
            Projection::Orthographic(o) => (
                "orthographic".into(),
                None,
                Some(GltfOrthographic {
                    xmag: o.x_mag,
                    ymag: o.y_mag,
                    znear: o.z_near,
                    zfar: o.z_far,
                }),
            ),
        };
        gltf.cameras.push(GltfCamera {
            name: if cam.name.is_empty() { None } else { Some(cam.name.clone()) },
            type_,
            perspective,
            orthographic,
        });
    }

    // --- Nodes ---
    let mut gltf_node_map: std::collections::HashMap<NodeId, usize> =
        std::collections::HashMap::new();
    let mut queue: std::collections::VecDeque<NodeId> =
        scene.roots.iter().cloned().collect();
    let mut ordered: Vec<NodeId> = Vec::new();
    {
        let mut visited = std::collections::HashSet::new();
        while let Some(nid) = queue.pop_front() {
            if !visited.insert(nid) {
                continue;
            }
            ordered.push(nid);
            if let Some(node) = scene.node(nid) {
                for &child in &node.children {
                    queue.push_back(child);
                }
            }
        }
    }
    for (gi, &nid) in ordered.iter().enumerate() {
        gltf_node_map.insert(nid, gi);
    }
    for &nid in &ordered {
        let node = scene.node(nid).unwrap();
        let t = &node.transform;
        let children: Vec<usize> = node
            .children
            .iter()
            .filter_map(|c| gltf_node_map.get(c).copied())
            .collect();
        let translation =
            if t.translation != Vec3::ZERO { Some(t.translation.to_array()) } else { None };
        let rotation = if t.rotation != Quat::IDENTITY {
            Some([t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w])
        } else {
            None
        };
        let scale = if t.scale != Vec3::ONE { Some(t.scale.to_array()) } else { None };
        let node_extensions = node.light.map(|li| {
            serde_json::json!({ "KHR_lights_punctual": { "light": li } })
        });
        gltf.nodes.push(GltfNode {
            name: if node.name.is_empty() { None } else { Some(node.name.clone()) },
            children,
            mesh:       node.mesh,
            camera:     node.camera,
            skin:       node.skin,
            translation,
            rotation,
            scale,
            extensions: node_extensions,
            ..Default::default()
        });
    }

    // --- Skins ---
    for skin in &scene.skins {
        let ibm_acc = if !skin.inverse_bind_matrices.is_empty() {
            let off = bin.len();
            for mat in &skin.inverse_bind_matrices {
                for &f in &mat.to_cols_array() {
                    bin.extend_from_slice(&f.to_le_bytes());
                }
            }
            let n_joints = skin.inverse_bind_matrices.len();
            let bv  = push_bv(&mut gltf, 0, off, n_joints * 64, None, None);
            let acc = push_acc(&mut gltf, bv, 5126, n_joints, "MAT4", 0);
            Some(acc)
        } else {
            None
        };
        let skeleton = skin.skeleton_root.and_then(|nid| gltf_node_map.get(&nid).copied());
        let joints: Vec<usize> = skin
            .joints
            .iter()
            .filter_map(|&nid| gltf_node_map.get(&nid).copied())
            .collect();
        gltf.skins.push(GltfSkin {
            name: if skin.name.is_empty() { None } else { Some(skin.name.clone()) },
            inverse_bind_matrices: ibm_acc,
            skeleton,
            joints,
        });
    }

    // --- Animations ---
    for anim in &scene.animations {
        let mut ganim = GltfAnimation {
            name: if anim.name.is_empty() { None } else { Some(anim.name.clone()) },
            ..Default::default()
        };
        for ch in &anim.channels {
            // Times accessor (SCALAR FLOAT)
            let times_off = bin.len();
            for &t in &ch.times {
                bin.extend_from_slice(&t.to_le_bytes());
            }
            let times_bv  = push_bv(&mut gltf, 0, times_off, ch.times.len() * 4, None, None);
            let times_acc = push_acc(&mut gltf, times_bv, 5126, ch.times.len(), "SCALAR", 0);
            if !ch.times.is_empty() {
                let acc = gltf.accessors.last_mut().unwrap();
                acc.min = vec![*ch.times.first().unwrap() as f64];
                acc.max = vec![*ch.times.last().unwrap() as f64];
            }

            // Values accessor
            let output_type = match &ch.target {
                AnimationTarget::Translation(_) | AnimationTarget::Scale(_) => "VEC3",
                AnimationTarget::Rotation(_) => "VEC4",
                AnimationTarget::MorphWeight { .. } => "SCALAR",
            };
            let n_comps: usize = match output_type {
                "VEC3" => 3, "VEC4" => 4, _ => 1,
            };
            let n_keyframes = if n_comps > 0 { ch.values.len() / n_comps } else { 0 };

            let vals_off = bin.len();
            for &v in &ch.values {
                bin.extend_from_slice(&v.to_le_bytes());
            }
            let vals_bv  = push_bv(&mut gltf, 0, vals_off, ch.values.len() * 4, None, None);
            let vals_acc = push_acc(&mut gltf, vals_bv, 5126, n_keyframes, output_type, 0);

            let interp = match ch.interpolation {
                Interpolation::Linear      => "LINEAR",
                Interpolation::Step        => "STEP",
                Interpolation::CubicSpline => "CUBICSPLINE",
            };

            let sampler_idx = ganim.samplers.len();
            ganim.samplers.push(GltfAnimationSampler {
                input:         times_acc,
                interpolation: Some(interp.into()),
                output:        vals_acc,
            });

            let (target_node, path) = match &ch.target {
                AnimationTarget::Translation(nid) => (gltf_node_map.get(nid).copied(), "translation"),
                AnimationTarget::Rotation(nid)    => (gltf_node_map.get(nid).copied(), "rotation"),
                AnimationTarget::Scale(nid)       => (gltf_node_map.get(nid).copied(), "scale"),
                AnimationTarget::MorphWeight { node_id, .. } => (gltf_node_map.get(node_id).copied(), "weights"),
            };
            ganim.channels.push(GltfAnimationChannel {
                sampler: sampler_idx,
                target:  GltfAnimationTarget { node: target_node, path: path.into() },
            });
        }
        gltf.animations.push(ganim);
    }

    // --- KHR_lights_punctual (save) ---
    if !scene.lights.is_empty() {
        gltf.extensions_used.push("KHR_lights_punctual".into());

        let lights_json: Vec<serde_json::Value> = scene.lights.iter().map(|light| {
            let color     = light.color();
            let intensity = light.intensity();
            let name      = light.name().to_string();
            let mut obj = serde_json::json!({
                "name":      name,
                "color":     [color.x, color.y, color.z],
                "intensity": intensity,
            });
            match light {
                Light::Directional(_) => {
                    obj["type"] = serde_json::Value::String("directional".into());
                }
                Light::Point(l) => {
                    obj["type"] = serde_json::Value::String("point".into());
                    if let Some(r) = l.range {
                        obj["range"] = serde_json::Value::from(r);
                    }
                }
                Light::Spot(l) => {
                    obj["type"] = serde_json::Value::String("spot".into());
                    if let Some(r) = l.range {
                        obj["range"] = serde_json::Value::from(r);
                    }
                    obj["spot"] = serde_json::json!({
                        "innerConeAngle": l.inner_cone_angle,
                        "outerConeAngle": l.outer_cone_angle,
                    });
                }
                Light::Area(_) => {
                    // No standard glTF equivalent; save as point light.
                    obj["type"] = serde_json::Value::String("point".into());
                }
            }
            obj
        }).collect();

        gltf.extensions = Some(serde_json::json!({
            "KHR_lights_punctual": { "lights": lights_json }
        }));
    }

    // --- Scene ---
    let root_gltf_indices: Vec<usize> = scene
        .roots
        .iter()
        .filter_map(|r| gltf_node_map.get(r).copied())
        .collect();
    gltf.scenes.push(GltfScene {
        name: if scene.name.is_empty() { None } else { Some(scene.name.clone()) },
        nodes: root_gltf_indices,
    });
    gltf.scene = Some(0);

    // Buffer 0: the binary blob (pushed last so byte_length is final)
    if !bin.is_empty() {
        gltf.buffers.push(GltfBuffer { byte_length: bin.len(), uri: None, name: None });
    }

    Ok((gltf, bin))
}

fn material_uses_specular_extension(material: &Material) -> bool {
    material.specular_color != Vec3::ONE
        || (material.specular_weight - DEFAULT_GLTF_SPECULAR_WEIGHT).abs() > GLTF_FLOAT_EPSILON
}

fn material_uses_ior_extension(material: &Material) -> bool {
    (material.ior - DEFAULT_GLTF_IOR).abs() > GLTF_FLOAT_EPSILON
}

fn build_material_extensions(material: &Material) -> Option<GltfMaterialExtensions> {
    let specular = material_uses_specular_extension(material).then(|| GltfMaterialsSpecular {
        specular_factor: ((material.specular_weight - DEFAULT_GLTF_SPECULAR_WEIGHT).abs()
            > GLTF_FLOAT_EPSILON)
            .then_some(material.specular_weight),
        specular_color_factor: (material.specular_color != Vec3::ONE).then_some([
            material.specular_color.x,
            material.specular_color.y,
            material.specular_color.z,
        ]),
    });
    let ior = material_uses_ior_extension(material)
        .then(|| GltfMaterialsIor { ior: Some(material.ior) });

    (specular.is_some() || ior.is_some()).then_some(GltfMaterialExtensions {
        khr_materials_specular: specular,
        khr_materials_ior: ior,
    })
}

fn push_bv(
    gltf: &mut GltfRoot,
    buffer: usize,
    offset: usize,
    length: usize,
    stride: Option<usize>,
    target: Option<u32>,
) -> usize {
    let idx = gltf.buffer_views.len();
    gltf.buffer_views.push(GltfBufferView {
        buffer,
        byte_offset: offset,
        byte_length: length,
        byte_stride: stride,
        target,
        ..Default::default()
    });
    idx
}

fn push_acc(
    gltf: &mut GltfRoot,
    bv: usize,
    component_type: u32,
    count: usize,
    type_: &str,
    byte_offset: usize,
) -> usize {
    let idx = gltf.accessors.len();
    gltf.accessors.push(GltfAccessor {
        buffer_view: Some(bv),
        component_type,
        count,
        type_: type_.into(),
        byte_offset,
        ..Default::default()
    });
    idx
}
