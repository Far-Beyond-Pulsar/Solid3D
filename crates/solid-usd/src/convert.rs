//! Bidirectional conversion between [`UsdDoc`] and [`Scene`].
//!
//! * [`doc_to_scene`] — walks the prim tree, expands USD Mesh prims into
//!   SolidRS [`Mesh`] objects, maps `UsdPreviewSurface` to [`Material`], and
//!   rebuilds the scene-graph hierarchy.
//! * [`scene_to_doc`] — converts a [`Scene`] back to a [`UsdDoc`] using
//!   `def Xform` / `def Mesh` / `def Material` prims and `UsdPreviewSurface`
//!   shaders.

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

use solid_rs::{
    SolidError,
    builder::SceneBuilder,
    geometry::{Primitive, SkinWeights, Transform, Vertex},
    scene::{
        AlphaMode, Animation, AnimationChannel, AnimationTarget, Camera, DirectionalLight,
        Image, ImageSource, Interpolation, Light, LightBase, Material, Mesh,
        OrthographicCamera, PerspectiveCamera, PointLight,
        Projection, Scene, Skin, SpotLight, Texture,
    },
};

use crate::document::{Attribute, Prim, Relationship, Specifier, StageMeta, UsdDoc, UsdValue};

// ── doc → scene ───────────────────────────────────────────────────────────────

/// Convert a [`UsdDoc`] into a SolidRS [`Scene`].
pub fn doc_to_scene(doc: &UsdDoc) -> Result<Scene, SolidError> {
    let mut ctx = LoadCtx {
        builder: SceneBuilder::new(),
        mat_path_to_idx: HashMap::new(),
        img_path_to_idx: HashMap::new(),
        tex_count: 0,
    };

    // ── Stage name ────────────────────────────────────────────────────────
    if let Some(dp) = &doc.meta.default_prim {
        ctx.builder = SceneBuilder::named(dp.as_str());
    }

    // ── Materials pass (collect all Material prims first) ────────────────
    for prim in &doc.root_prims {
        collect_materials(prim, "/", &mut ctx)?;
    }

    // ── Scene graph pass ──────────────────────────────────────────────────
    for prim in &doc.root_prims {
        visit_prim(prim, "/", None, &mut ctx)?;
    }

    let mut scene = ctx.builder.build();
    scene.metadata.source_format = Some("USDA 1.0".into());
    if let Some(axis) = &doc.meta.up_axis {
        scene.metadata.extra.insert("upAxis".into(), solid_rs::value::Value::String(axis.clone()));
    }

    Ok(scene)
}

// ── Loading context ───────────────────────────────────────────────────────────

struct LoadCtx {
    builder:         SceneBuilder,
    mat_path_to_idx: HashMap<String, usize>,
    img_path_to_idx: HashMap<String, usize>,
    tex_count:       usize,
}

// ── Material collection ───────────────────────────────────────────────────────

fn collect_materials(prim: &Prim, parent_path: &str, ctx: &mut LoadCtx) -> Result<(), SolidError> {
    let path = prim.path(parent_path);

    if prim.type_name == "Material" {
        let mat = build_material(prim, &path, ctx);
        let idx = ctx.builder.push_material(mat);
        ctx.mat_path_to_idx.insert(path.clone(), idx);
    }

    for child in &prim.children {
        collect_materials(child, &path, ctx)?;
    }
    Ok(())
}

fn build_material(prim: &Prim, _path: &str, ctx: &mut LoadCtx) -> Material {
    let mut mat = Material::new(prim.name.clone());

    // Find the UsdPreviewSurface shader child
    let shader = prim.children.iter().find(|c| c.type_name == "Shader");

    if let Some(shader) = shader {
        if let Some(attr) = shader.attr("inputs:diffuseColor") {
            if let Some(UsdValue::Vec3f([r, g, b])) = &attr.value {
                mat.base_color_factor = Vec4::new(*r as f32, *g as f32, *b as f32, 1.0);
            }
        }
        if let Some(attr) = shader.attr("inputs:emissiveColor") {
            if let Some(UsdValue::Vec3f([r, g, b])) = &attr.value {
                mat.emissive_factor = Vec3::new(*r as f32, *g as f32, *b as f32);
            }
        }
        if let Some(attr) = shader.attr("inputs:roughness") {
            if let Some(v) = attr_as_f32(attr) { mat.roughness_factor = v; }
        }
        if let Some(attr) = shader.attr("inputs:metallic") {
            if let Some(v) = attr_as_f32(attr) { mat.metallic_factor = v; }
        }
        if let Some(attr) = shader.attr("inputs:opacity") {
            let opacity = attr_as_f32(attr).unwrap_or(1.0);
            if opacity < 1.0 { mat.alpha_mode = AlphaMode::Blend; }
        }
        if let Some(attr) = shader.attr("inputs:doubleSided") {
            if let Some(UsdValue::Bool(v)) = &attr.value { mat.double_sided = *v; }
        }
        // Diffuse texture via a UVTexture shader child
        if let Some(tex_shader) = prim.children.iter()
            .find(|c| c.type_name == "Shader" && c.name != shader.name)
        {
            if let Some(file_attr) = tex_shader.attr("inputs:file") {
                if let Some(UsdValue::Asset(path)) = &file_attr.value {
                    let img_idx = *ctx.img_path_to_idx.entry(path.clone()).or_insert_with(|| {
                        let img = Image {
                            name:       path.clone(),
                            source:     ImageSource::Uri(path.clone()),
                            extensions: Default::default(),
                        };
                        ctx.builder.push_image(img)
                    });
                    let tex = Texture {
                        name:        path.clone(),
                        image_index: img_idx,
                        sampler:     Default::default(),
                        extensions:  Default::default(),
                    };
                    let tex_idx = ctx.builder.push_texture(tex);
                    ctx.tex_count += 1;
                    mat.base_color_texture = Some(solid_rs::scene::TextureRef::new(tex_idx));
                }
            }
        }
    }

    mat
}

// ── Prim visitor ─────────────────────────────────────────────────────────────

fn visit_prim(
    prim:        &Prim,
    parent_path: &str,
    parent_id:   Option<solid_rs::scene::NodeId>,
    ctx:         &mut LoadCtx,
) -> Result<(), SolidError> {
    let path = prim.path(parent_path);

    // Skip pure Material subtrees — already processed
    if prim.type_name == "Material" {
        return Ok(());
    }

    let node_id = match parent_id {
        None    => ctx.builder.add_root_node(prim.name.clone()),
        Some(p) => ctx.builder.add_child_node(p, prim.name.clone()),
    };

    // Transform
    let xform = extract_xform(prim);
    ctx.builder.set_transform(node_id, xform);

    // Mesh geometry
    if prim.type_name == "Mesh" {
        if let Some(mesh) = build_mesh(prim, &path, ctx)? {
            let mi = ctx.builder.push_mesh(mesh);
            ctx.builder.attach_mesh(node_id, mi);
        }
    }

    // Camera
    if prim.type_name == "Camera" {
        if let Some(cam) = build_camera(prim) {
            let ci = ctx.builder.push_camera(cam);
            ctx.builder.attach_camera(node_id, ci);
        }
    }

    // Light
    if prim.type_name.ends_with("Light") {
        if let Some(light) = build_light(prim) {
            let li = ctx.builder.push_light(light);
            ctx.builder.attach_light(node_id, li);
        }
    }

    for child in &prim.children {
        visit_prim(child, &path, Some(node_id), ctx)?;
    }

    Ok(())
}

// ── Mesh construction ─────────────────────────────────────────────────────────

fn build_mesh(prim: &Prim, path: &str, ctx: &mut LoadCtx) -> Result<Option<Mesh>, SolidError> {
    let points = match prim.attr("points") {
        Some(a) => match &a.value {
            Some(UsdValue::Vec3fArray(v)) => v.clone(),
            _ => return Ok(None),
        },
        None => return Ok(None),
    };

    let face_counts = prim.attr("faceVertexCounts")
        .and_then(|a| if let Some(UsdValue::IntArray(v)) = &a.value { Some(v.clone()) } else { None })
        .unwrap_or_default();

    let face_indices = prim.attr("faceVertexIndices")
        .and_then(|a| if let Some(UsdValue::IntArray(v)) = &a.value { Some(v.clone()) } else { None })
        .unwrap_or_default();

    // Optional per-vertex attributes
    let normals   = extract_vec3_array(prim, "normals");
    let uvs       = extract_vec2_array(prim, "primvars:st")
        .or_else(|| extract_vec2_array(prim, "primvars:UVMap"))
        .or_else(|| extract_vec2_array(prim, "primvars:uv"));
    let uv_indices = prim.attr("primvars:st:indices")
        .or_else(|| prim.attr("primvars:UVMap:indices"))
        .or_else(|| prim.attr("primvars:uv:indices"))
        .and_then(|a| if let Some(UsdValue::IntArray(v)) = &a.value { Some(v.clone()) } else { None });

    let normal_indices = prim.attr("normals:indices")
        .and_then(|a| if let Some(UsdValue::IntArray(v)) = &a.value { Some(v.clone()) } else { None });

    // Build interleaved vertex buffer by expanding per-face indices
    let mut mesh = Mesh::new(prim.name.clone());

    // Determine material binding
    let mat_idx = prim.rel("material:binding")
        .and_then(|r| r.target.as_deref())
        .and_then(|p| ctx.mat_path_to_idx.get(p))
        .copied();

    // Expand face-indexed geometry into a flat triangle list
    let mut flat_indices: Vec<u32> = Vec::new();
    let mut face_start = 0usize;

    for &count in &face_counts {
        let count = count as usize;
        let face_verts: Vec<usize> = (0..count)
            .map(|i| face_indices[face_start + i] as usize)
            .collect();

        // Fan-triangulate the face
        for i in 1..(count - 1) {
            for &vi in &[0, i, i + 1] {
                let geom_idx = face_verts[vi];
                let normal   = normals.as_ref().map(|ns| {
                    let idx = normal_indices.as_ref().map_or(face_start + vi, |ni| ni[face_start + vi] as usize);
                    let n = ns.get(idx).copied().unwrap_or([0.0, 1.0, 0.0]);
                    Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32)
                });
                let uv = uvs.as_ref().and_then(|us| {
                    let idx = uv_indices.as_ref().map_or(face_start + vi, |ui| ui[face_start + vi] as usize);
                    us.get(idx).map(|u| Vec2::new(u[0] as f32, u[1] as f32))
                });

                let pos = points.get(geom_idx).copied().unwrap_or([0.0, 0.0, 0.0]);
                let mut v = Vertex::new(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
                if let Some(n) = normal { v.normal = Some(n); }
                if let Some(u) = uv     { v.uvs[0] = Some(u); }

                let vert_idx = mesh.vertices.len() as u32;
                mesh.vertices.push(v);
                flat_indices.push(vert_idx);
            }
        }

        face_start += count;
    }

    if !flat_indices.is_empty() {
        mesh.primitives.push(Primitive::triangles(flat_indices, mat_idx));
    }

    // Warn but don't fail on empty meshes — USD allows placeholder prims
    if mesh.is_empty() {
        return Ok(None);
    }

    let _ = path; // used for context in future extensions
    Ok(Some(mesh))
}

// ── Camera ────────────────────────────────────────────────────────────────────

fn build_camera(prim: &Prim) -> Option<Camera> {
    let projection_type = prim.attr("projection")
        .and_then(|a| if let Some(UsdValue::Token(t)) = &a.value { Some(t.as_str()) } else { None })
        .unwrap_or("perspective");

    let projection = if projection_type == "orthographic" {
        let h_aperture = attr_f64(prim, "horizontalAperture").unwrap_or(20.955);
        let v_aperture = attr_f64(prim, "verticalAperture").unwrap_or(15.2908);
        Projection::Orthographic(OrthographicCamera {
            x_mag:  h_aperture as f32,
            y_mag:  v_aperture as f32,
            z_near: attr_f64(prim, "clippingRange").map_or(0.1, |v| v) as f32,
            z_far:  100.0,
        })
    } else {
        let fov = attr_f64(prim, "fov")
            .or_else(|| {
                // compute from horizontalAperture + focalLength
                let ha = attr_f64(prim, "horizontalAperture")?;
                let fl = attr_f64(prim, "focalLength")?;
                Some(2.0 * (ha / (2.0 * fl)).atan())
            })
            .unwrap_or(0.785398) as f32;
        Projection::Perspective(PerspectiveCamera {
            fov_y:        fov,
            aspect_ratio: None,
            z_near:       0.1,
            z_far:        Some(1000.0),
        })
    };

    Some(Camera {
        name:       prim.name.clone(),
        projection,
        extensions: Default::default(),
    })
}

// ── Lights ────────────────────────────────────────────────────────────────────

fn build_light(prim: &Prim) -> Option<Light> {
    let color = extract_color(prim, "inputs:color")
        .or_else(|| extract_color(prim, "color"))
        .unwrap_or(Vec3::ONE);
    let intensity = attr_f64(prim, "inputs:intensity")
        .or_else(|| attr_f64(prim, "intensity"))
        .unwrap_or(1.0) as f32;
    let base = LightBase { name: prim.name.clone(), color, intensity };

    let light = match prim.type_name.as_str() {
        "PointLight" | "SphereLight" => Light::Point(PointLight {
            base,
            range:      attr_f64(prim, "inputs:radius").map(|r| r as f32),
            extensions: Default::default(),
        }),
        "DistantLight" => Light::Directional(DirectionalLight {
            base,
            extensions: Default::default(),
        }),
        "SpotLight" => {
            let inner = attr_f64(prim, "inputs:shaping:cone:softness")
                .map(|v| v as f32).unwrap_or(0.0);
            let outer = attr_f64(prim, "inputs:shaping:cone:angle")
                .map(|v| v.to_radians() as f32).unwrap_or(std::f32::consts::FRAC_PI_4);
            Light::Spot(solid_rs::scene::SpotLight {
                base,
                range:             None,
                inner_cone_angle:  inner,
                outer_cone_angle:  outer,
                extensions:        Default::default(),
            })
        }
        _ => Light::Directional(DirectionalLight { base, extensions: Default::default() }),
    };

    Some(light)
}

// ── Transform extraction ──────────────────────────────────────────────────────

fn extract_xform(prim: &Prim) -> Transform {
    let translation = extract_vec3(prim, "xformOp:translate")
        .map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32))
        .unwrap_or(Vec3::ZERO);

    let scale = extract_vec3(prim, "xformOp:scale")
        .map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32))
        .unwrap_or(Vec3::ONE);

    let rotation = extract_vec3(prim, "xformOp:rotateXYZ")
        .map(|v| {
            let rx = Quat::from_rotation_x((v[0] as f32).to_radians());
            let ry = Quat::from_rotation_y((v[1] as f32).to_radians());
            let rz = Quat::from_rotation_z((v[2] as f32).to_radians());
            rz * ry * rx
        })
        .unwrap_or(Quat::IDENTITY);

    Transform { translation, rotation, scale }
}

// ── scene → doc ───────────────────────────────────────────────────────────────

/// Convert a SolidRS [`Scene`] into a [`UsdDoc`].
pub fn scene_to_doc(scene: &Scene) -> Result<UsdDoc, SolidError> {
    let mut doc = UsdDoc::new();

    // Stage metadata
    doc.meta = StageMeta {
        up_axis:         Some("Y".into()),
        meters_per_unit: Some(1.0),
        default_prim:    if scene.name.is_empty() { Some("Root".into()) } else { Some(scene.name.clone()) },
        doc:             None,
        extra:           HashMap::new(),
    };

    // ── Materials ────────────────────────────────────────────────────────
    let mut mat_prims: Vec<Prim> = Vec::new();
    for (i, mat) in scene.materials.iter().enumerate() {
        mat_prims.push(build_material_prim(mat, i, scene));
    }

    // ── Scene-graph prim ─────────────────────────────────────────────────
    let root_name = if scene.name.is_empty() { "Root" } else { &scene.name };
    let mut root_prim = Prim::new(Specifier::Def, "Xform", root_name);

    // Embed materials under root
    for mp in mat_prims {
        root_prim.children.push(mp);
    }

    // Meshes and hierarchy
    for &root_id in &scene.roots {
        if let Some(node) = scene.node(root_id) {
            let child = build_node_prim(node, scene);
            root_prim.children.push(child);
        }
    }

    doc.root_prims.push(root_prim);
    Ok(doc)
}

fn build_material_prim(mat: &Material, idx: usize, scene: &Scene) -> Prim {
    let mat_name = sanitise_name(if mat.name.is_empty() {
        format!("Material{idx}")
    } else {
        mat.name.clone()
    });

    let mut mat_prim = Prim::new(Specifier::Def, "Material", &mat_name);

    // UsdPreviewSurface shader
    let shader_name = format!("{mat_name}_Shader");
    let mut shader = Prim::new(Specifier::Def, "Shader", &shader_name);

    shader.attributes.push(Attribute::new("info:id", "token", UsdValue::Token("UsdPreviewSurface".into())).uniform());

    let [r, g, b, _] = mat.base_color_factor.to_array();
    shader.attributes.push(Attribute::new(
        "inputs:diffuseColor", "color3f",
        UsdValue::Vec3f([r as f64, g as f64, b as f64]),
    ));

    let [er, eg, eb] = mat.emissive_factor.to_array();
    shader.attributes.push(Attribute::new(
        "inputs:emissiveColor", "color3f",
        UsdValue::Vec3f([er as f64, eg as f64, eb as f64]),
    ));

    shader.attributes.push(Attribute::new(
        "inputs:roughness", "float",
        UsdValue::Float(mat.roughness_factor as f64),
    ));
    shader.attributes.push(Attribute::new(
        "inputs:metallic", "float",
        UsdValue::Float(mat.metallic_factor as f64),
    ));
    shader.attributes.push(Attribute::new(
        "inputs:doubleSided", "bool",
        UsdValue::Bool(mat.double_sided),
    ));

    // Texture
    if let Some(tex_ref) = &mat.base_color_texture {
        if let Some(tex) = scene.textures.get(tex_ref.texture_index) {
            if let Some(img) = scene.images.get(tex.image_index) {
                let uri = match &img.source {
                    ImageSource::Uri(u)  => u.clone(),
                    ImageSource::Embedded { .. } => format!("{}_{}.png", mat_name, tex.image_index),
                };
                let tex_shader_name = format!("{mat_name}_DiffuseTex");
                let mut tex_shader = Prim::new(Specifier::Def, "Shader", &tex_shader_name);
                tex_shader.attributes.push(
                    Attribute::new("info:id", "token", UsdValue::Token("UsdUVTexture".into())).uniform()
                );
                tex_shader.attributes.push(Attribute::new("inputs:file", "asset", UsdValue::Asset(uri)));
                mat_prim.children.push(tex_shader);
            }
        }
    }

    mat_prim.children.push(shader);

    // surface output
    mat_prim.relationships.push(Relationship {
        name:   "outputs:surface".into(),
        target: Some(format!("/Root/{mat_name}/{shader_name}.outputs:surface")),
    });

    mat_prim
}

fn build_node_prim(node: &solid_rs::scene::Node, scene: &Scene) -> Prim {
    let node_name = sanitise_name(if node.name.is_empty() {
        format!("Node{}", node.id.0)
    } else {
        node.name.clone()
    });

    let prim_type = if node.mesh.is_some() { "Mesh" } else { "Xform" };
    let mut prim = Prim::new(Specifier::Def, prim_type, &node_name);

    // Transform
    let tf = &node.transform;
    if !tf.is_identity() {
        prim.attributes.push(Attribute::new(
            "xformOp:translate", "double3",
            UsdValue::Vec3f([
                tf.translation.x as f64,
                tf.translation.y as f64,
                tf.translation.z as f64,
            ]),
        ));
        let (ax, ay, az) = tf.rotation.to_euler(glam::EulerRot::XYZ);
        prim.attributes.push(Attribute::new(
            "xformOp:rotateXYZ", "float3",
            UsdValue::Vec3f([
                ax.to_degrees() as f64,
                ay.to_degrees() as f64,
                az.to_degrees() as f64,
            ]),
        ));
        prim.attributes.push(Attribute::new(
            "xformOp:scale", "float3",
            UsdValue::Vec3f([tf.scale.x as f64, tf.scale.y as f64, tf.scale.z as f64]),
        ));
        prim.attributes.push(Attribute::new(
            "xformOpOrder", "token[]",
            UsdValue::TokenArray(vec![
                "xformOp:translate".into(),
                "xformOp:rotateXYZ".into(),
                "xformOp:scale".into(),
            ]),
        ));
    }

    // Mesh data
    if let Some(mesh_idx) = node.mesh {
        if let Some(mesh) = scene.meshes.get(mesh_idx) {
            write_mesh_attrs(&mut prim, mesh, scene);
        }
    }

    // Children
    for &child_id in &node.children {
        if let Some(child) = scene.node(child_id) {
            prim.children.push(build_node_prim(child, scene));
        }
    }

    prim
}

fn write_mesh_attrs(prim: &mut Prim, mesh: &Mesh, scene: &Scene) {
    // Points
    let points: Vec<[f64; 3]> = mesh.vertices.iter()
        .map(|v| [v.position.x as f64, v.position.y as f64, v.position.z as f64])
        .collect();
    prim.attributes.push(Attribute::new("points", "point3f[]", UsdValue::Vec3fArray(points)));

    // Normals
    let has_normals = mesh.vertices.iter().any(|v| v.normal.is_some());
    if has_normals {
        let normals: Vec<[f64; 3]> = mesh.vertices.iter()
            .map(|v| {
                let n = v.normal.unwrap_or(Vec3::Y);
                [n.x as f64, n.y as f64, n.z as f64]
            })
            .collect();
        prim.attributes.push(Attribute::new("normals", "normal3f[]", UsdValue::Vec3fArray(normals)));
    }

    // UVs
    let has_uvs = mesh.vertices.iter().any(|v| v.uvs[0].is_some());
    if has_uvs {
        let uvs: Vec<[f64; 3]> = mesh.vertices.iter()
            .map(|v| {
                let uv = v.uvs[0].unwrap_or(Vec2::ZERO);
                [uv.x as f64, uv.y as f64, 0.0]
            })
            .collect();
        prim.attributes.push(Attribute::new("primvars:st", "texCoord2f[]", UsdValue::Vec3fArray(uvs)));
    }

    // Face topology — reconstruct from flat index buffer
    let (face_counts, face_indices, mat_rel) = mesh_to_faces(mesh);
    prim.attributes.push(Attribute::new("faceVertexCounts", "int[]", UsdValue::IntArray(face_counts)));
    prim.attributes.push(Attribute::new("faceVertexIndices", "int[]", UsdValue::IntArray(face_indices)));
    prim.attributes.push(Attribute::new(
        "subdivisionScheme", "token",
        UsdValue::Token("none".into()),
    ));

    // Material binding
    if let Some(mat_idx) = mat_rel {
        if let Some(mat) = scene.materials.get(mat_idx) {
            let mat_name = sanitise_name(if mat.name.is_empty() {
                format!("Material{mat_idx}")
            } else {
                mat.name.clone()
            });
            // Assuming root prim is "Root"
            let root_name = "Root";
            prim.relationships.push(Relationship {
                name:   "material:binding".into(),
                target: Some(format!("/{root_name}/{mat_name}")),
            });
        }
    }
}

/// Reconstruct face topology from a flat triangle list.
/// Returns (faceVertexCounts, faceVertexIndices, optional_material_index).
fn mesh_to_faces(mesh: &Mesh) -> (Vec<i64>, Vec<i64>, Option<usize>) {
    let mut face_counts  = Vec::new();
    let mut face_indices = Vec::new();
    let mut mat_idx      = None;

    for prim in &mesh.primitives {
        if mat_idx.is_none() {
            mat_idx = prim.material_index;
        }
        // Each group of 3 indices → one triangle face
        let mut i = 0;
        while i + 2 < prim.indices.len() {
            face_counts.push(3);
            face_indices.push(prim.indices[i]   as i64);
            face_indices.push(prim.indices[i+1] as i64);
            face_indices.push(prim.indices[i+2] as i64);
            i += 3;
        }
    }

    (face_counts, face_indices, mat_idx)
}

// ── Attribute helpers ─────────────────────────────────────────────────────────

fn extract_vec3_array(prim: &Prim, name: &str) -> Option<Vec<[f64; 3]>> {
    if let Some(UsdValue::Vec3fArray(v)) = prim.attr(name).and_then(|a| a.value.as_ref()) {
        Some(v.clone())
    } else {
        None
    }
}

fn extract_vec2_array(prim: &Prim, name: &str) -> Option<Vec<[f64; 2]>> {
    if let Some(UsdValue::Vec2fArray(v)) = prim.attr(name).and_then(|a| a.value.as_ref()) {
        Some(v.clone())
    } else if let Some(UsdValue::Vec3fArray(v)) = prim.attr(name).and_then(|a| a.value.as_ref()) {
        Some(v.iter().map(|p| [p[0], p[1]]).collect())
    } else {
        None
    }
}

fn extract_vec3(prim: &Prim, name: &str) -> Option<[f64; 3]> {
    match prim.attr(name).and_then(|a| a.value.as_ref()) {
        Some(UsdValue::Vec3f(v)) => Some(*v),
        _ => None,
    }
}

fn attr_as_f32(attr: &Attribute) -> Option<f32> {
    match &attr.value {
        Some(UsdValue::Float(f)) => Some(*f as f32),
        Some(UsdValue::Int(i))   => Some(*i as f32),
        _ => None,
    }
}

fn attr_f64(prim: &Prim, name: &str) -> Option<f64> {
    match prim.attr(name).and_then(|a| a.value.as_ref()) {
        Some(UsdValue::Float(f)) => Some(*f),
        Some(UsdValue::Int(i))   => Some(*i as f64),
        Some(UsdValue::Vec2f(v)) => Some(v[0]), // e.g. clippingRange takes near
        _ => None,
    }
}

fn extract_color(prim: &Prim, name: &str) -> Option<Vec3> {
    if let Some(UsdValue::Vec3f([r, g, b])) = prim.attr(name).and_then(|a| a.value.as_ref()) {
        Some(Vec3::new(*r as f32, *g as f32, *b as f32))
    } else {
        None
    }
}

/// Replace characters that are invalid in USD prim names.
fn sanitise_name(s: String) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}
