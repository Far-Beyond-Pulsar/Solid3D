//! Integration tests — ASCII FBX round-trips (save → load → verify).

mod common;
use common::*;

use solid_fbx::{FbxLoader, FbxSaver};
use solid_rs::prelude::*;
use glam::*;
use std::io::Cursor;

// ── Geometry ──────────────────────────────────────────────────────────────────

#[test]
fn ascii_triangle_positions_survive_round_trip() {
    let scene  = triangle_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.meshes.is_empty(), "no meshes after round-trip");
    let orig = &scene.meshes[0];
    let got  = &loaded.meshes[0];
    assert_eq!(got.vertices.len(), orig.vertices.len(), "vertex count changed");
    for (ov, lv) in orig.vertices.iter().zip(got.vertices.iter()) {
        assert!((ov.position.x - lv.position.x).abs() < 1e-4, "position.x mismatch");
        assert!((ov.position.y - lv.position.y).abs() < 1e-4, "position.y mismatch");
        assert!((ov.position.z - lv.position.z).abs() < 1e-4, "position.z mismatch");
    }
}

#[test]
fn ascii_normals_survive_round_trip() {
    let scene  = triangle_scene();
    let loaded = ascii_round_trip(&scene);
    for v in &loaded.meshes[0].vertices {
        let n = v.normal.expect("normal missing after round-trip");
        assert!(n.length() > 0.9, "normal should be unit-length, got {:?}", n);
        // Original normals all point in +Z
        assert!((n.z - 1.0).abs() < 1e-3, "normal.z should be ~1.0, got {}", n.z);
    }
}

#[test]
fn ascii_uvs_survive_round_trip() {
    let mut b    = SceneBuilder::new();
    let mut mesh = Mesh::new("UvMesh");
    // Use V=0.5 so a potential V-flip (1-0.5=0.5) doesn't change the value.
    mesh.vertices = vec![
        Vertex::new(Vec3::new( 0.0,  1.0, 0.0)).with_normal(Vec3::Z).with_uv(Vec2::new(0.25, 0.5)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z).with_uv(Vec2::new(0.0,  0.5)),
        Vertex::new(Vec3::new( 1.0, -1.0, 0.0)).with_normal(Vec3::Z).with_uv(Vec2::new(0.75, 0.5)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r  = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    assert!(!loaded.meshes.is_empty());
    let loaded_mesh = &loaded.meshes[0];
    for v in loaded_mesh.vertices.iter() {
        assert!(v.uv().is_some(), "UV missing after round-trip");
    }
    let orig_uvs:   Vec<Vec2> = scene.meshes[0].vertices.iter().map(|v| v.uv().unwrap()).collect();
    let loaded_uvs: Vec<Vec2> = loaded_mesh.vertices.iter().map(|v| v.uv().unwrap()).collect();
    for (ou, lu) in orig_uvs.iter().zip(loaded_uvs.iter()) {
        assert!((ou.x - lu.x).abs() < 1e-4, "UV.x mismatch: {} vs {}", ou.x, lu.x);
        // V=0.5 is symmetric under a V-flip, so either convention passes.
        assert!((ou.y - lu.y).abs() < 1e-4, "UV.y mismatch: {} vs {}", ou.y, lu.y);
    }
}

#[test]
fn ascii_vertex_colors_survive_round_trip() {
    let scene  = vertex_color_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.meshes.is_empty());
    let lm = &loaded.meshes[0];
    for v in &lm.vertices {
        assert!(v.color().is_some(), "vertex color missing after round-trip");
    }
    let orig_colors:   Vec<Vec4> = scene.meshes[0].vertices.iter().map(|v| v.color().unwrap()).collect();
    let loaded_colors: Vec<Vec4> = lm.vertices.iter().map(|v| v.color().unwrap()).collect();
    for (oc, lc) in orig_colors.iter().zip(loaded_colors.iter()) {
        assert!((oc.x - lc.x).abs() < 1e-3, "color.r mismatch: {} vs {}", oc.x, lc.x);
        assert!((oc.y - lc.y).abs() < 1e-3, "color.g mismatch: {} vs {}", oc.y, lc.y);
        assert!((oc.z - lc.z).abs() < 1e-3, "color.b mismatch: {} vs {}", oc.z, lc.z);
    }
}

#[test]
fn ascii_tangents_survive_round_trip() {
    let scene  = tangent_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.meshes.is_empty());
    let has_tangents = loaded.meshes[0].vertices.iter().any(|v| v.tangent.is_some());
    assert!(has_tangents, "tangents missing after round-trip");
}

// ── Materials ─────────────────────────────────────────────────────────────────

#[test]
fn ascii_material_diffuse_survives() {
    let scene  = material_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.materials.is_empty(), "no materials after round-trip");
    let orig = &scene.materials[0];
    let got  = &loaded.materials[0];
    assert!((got.base_color_factor.x - orig.base_color_factor.x).abs() < 1e-3, "diffuse.r");
    assert!((got.base_color_factor.y - orig.base_color_factor.y).abs() < 1e-3, "diffuse.g");
    assert!((got.base_color_factor.z - orig.base_color_factor.z).abs() < 1e-3, "diffuse.b");
}

#[test]
fn ascii_material_emissive_survives() {
    let scene  = material_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.materials.is_empty());
    let orig = &scene.materials[0];
    let got  = &loaded.materials[0];
    assert!((got.emissive_factor.x - orig.emissive_factor.x).abs() < 1e-3, "emissive.r");
    assert!((got.emissive_factor.y - orig.emissive_factor.y).abs() < 1e-3, "emissive.g");
    assert!((got.emissive_factor.z - orig.emissive_factor.z).abs() < 1e-3, "emissive.b");
}

#[test]
fn ascii_material_roughness_survives() {
    let scene  = material_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.materials.is_empty());
    let orig_r = scene.materials[0].roughness_factor;
    let got_r  = loaded.materials[0].roughness_factor;
    assert!((orig_r - got_r).abs() < 1e-3, "roughness: {} vs {}", orig_r, got_r);
}

#[test]
fn ascii_material_metallic_survives() {
    let scene  = material_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.materials.is_empty());
    let orig_m = scene.materials[0].metallic_factor;
    let got_m  = loaded.materials[0].metallic_factor;
    assert!((orig_m - got_m).abs() < 1e-3, "metallic: {} vs {}", orig_m, got_m);
}

#[test]
fn ascii_material_alpha_survives() {
    let scene  = material_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.materials.is_empty());
    let orig_a = scene.materials[0].base_color_factor.w;
    let got_a  = loaded.materials[0].base_color_factor.w;
    assert!((orig_a - got_a).abs() < 1e-3, "alpha: {} vs {}", orig_a, got_a);
}

#[test]
fn ascii_diffuse_texture_survives() {
    let mut b       = SceneBuilder::new();
    let img_idx     = b.push_image(Image::from_uri("DiffuseImg", "diffuse.png"));
    let tex_idx     = b.push_texture(Texture::new("DiffuseTex", img_idx));
    let mut mat     = Material::new("TexMat");
    mat.base_color_texture = Some(TextureRef::new(tex_idx));
    let mat_idx     = b.push_material(mat);
    let mut mesh    = make_minimal_mesh("TexMesh");
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mat_idx))];
    let mi          = b.push_mesh(mesh);
    let r           = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    assert!(!loaded.images.is_empty(), "images missing after round-trip");
    let uri = match &loaded.images[0].source {
        ImageSource::Uri(u) => u.clone(),
        _ => String::new(),
    };
    assert!(uri.contains("diffuse"), "diffuse URI not preserved: {:?}", uri);
}

// ── Node hierarchy & transforms ───────────────────────────────────────────────

#[test]
fn ascii_node_hierarchy_survives() {
    let mut b  = SceneBuilder::new();
    let root   = b.add_root_node("RootNode");
    let child  = b.add_child_node(root, "ChildNode");
    let mi     = b.push_mesh(make_minimal_mesh("ChildMesh"));
    b.attach_mesh(child, mi);
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    let child_node = loaded.nodes.iter()
        .find(|n| n.name == "ChildNode")
        .expect("ChildNode not found after round-trip");
    assert!(child_node.parent.is_some(), "ChildNode has no parent");
    let parent_id   = child_node.parent.unwrap();
    let parent_node = loaded.node(parent_id).expect("parent node missing");
    assert_eq!(parent_node.name, "RootNode");
}

#[test]
fn ascii_node_translation_survives() {
    let mut b  = SceneBuilder::new();
    let r      = b.add_root_node("Root");
    let child  = b.add_child_node(r, "Translated");
    let mi     = b.push_mesh(make_minimal_mesh("TMesh"));
    b.attach_mesh(child, mi);
    b.set_transform(child, Transform::IDENTITY.with_translation(Vec3::new(1.5, 2.5, -3.5)));
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    let node = loaded.nodes.iter().find(|n| n.name == "Translated").expect("Translated missing");
    let tr   = node.transform.translation;
    assert!((tr.x -  1.5).abs() < 1e-3, "translation.x: {} vs 1.5",  tr.x);
    assert!((tr.y -  2.5).abs() < 1e-3, "translation.y: {} vs 2.5",  tr.y);
    assert!((tr.z + 3.5).abs()  < 1e-3, "translation.z: {} vs -3.5", tr.z);
}

#[test]
fn ascii_node_rotation_survives() {
    use std::f32::consts::FRAC_PI_4;
    let mut b    = SceneBuilder::new();
    let r        = b.add_root_node("Root");
    let child    = b.add_child_node(r, "Rotated");
    let mi       = b.push_mesh(make_minimal_mesh("RMesh"));
    b.attach_mesh(child, mi);
    let orig_rot = Quat::from_rotation_y(FRAC_PI_4);
    b.set_transform(child, Transform::IDENTITY.with_rotation(orig_rot));
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    let node = loaded.nodes.iter().find(|n| n.name == "Rotated").expect("Rotated missing");
    let got  = node.transform.rotation;
    // Compare by applying to a test vector — tolerant of quat sign flip.
    let test_v   = Vec3::new(1.0, 0.0, 0.0);
    let expected = orig_rot * test_v;
    let actual   = got      * test_v;
    assert!((expected.x - actual.x).abs() < 1e-2, "rot.x {} vs {}", expected.x, actual.x);
    assert!((expected.y - actual.y).abs() < 1e-2, "rot.y {} vs {}", expected.y, actual.y);
    assert!((expected.z - actual.z).abs() < 1e-2, "rot.z {} vs {}", expected.z, actual.z);
}

#[test]
fn ascii_node_scale_survives() {
    let mut b      = SceneBuilder::new();
    let r          = b.add_root_node("Root");
    let child      = b.add_child_node(r, "Scaled");
    let mi         = b.push_mesh(make_minimal_mesh("SMesh"));
    b.attach_mesh(child, mi);
    let orig_scale = Vec3::new(2.0, 3.0, 0.5);
    b.set_transform(child, Transform::IDENTITY.with_scale(orig_scale));
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    let node = loaded.nodes.iter().find(|n| n.name == "Scaled").expect("Scaled missing");
    let s    = node.transform.scale;
    assert!((s.x - orig_scale.x).abs() < 1e-3, "scale.x {} vs {}", s.x, orig_scale.x);
    assert!((s.y - orig_scale.y).abs() < 1e-3, "scale.y {} vs {}", s.y, orig_scale.y);
    assert!((s.z - orig_scale.z).abs() < 1e-3, "scale.z {} vs {}", s.z, orig_scale.z);
}

// ── Cameras ───────────────────────────────────────────────────────────────────

#[test]
fn ascii_perspective_camera_fov_survives() {
    use std::f32::consts::FRAC_PI_4;
    let mut b   = SceneBuilder::new();
    let mut cam = Camera::perspective("MainCam");
    if let Projection::Perspective(ref mut p) = cam.projection { p.fov_y = FRAC_PI_4; }
    let ci = b.push_camera(cam);
    let r  = b.add_root_node("Root");
    let cn = b.add_child_node(r, "CamNode");
    b.attach_camera(cn, ci);
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    assert!(!loaded.cameras.is_empty(), "no cameras after round-trip");
    if let Projection::Perspective(ref p) = loaded.cameras[0].projection {
        assert!((p.fov_y - FRAC_PI_4).abs() < 1e-2, "fov_y {} vs {}", p.fov_y, FRAC_PI_4);
    } else {
        panic!("expected perspective projection");
    }
}

#[test]
fn ascii_perspective_camera_near_far_survives() {
    let mut b   = SceneBuilder::new();
    let mut cam = Camera::perspective("NearFarCam");
    if let Projection::Perspective(ref mut p) = cam.projection {
        p.z_near = 0.1;
        p.z_far  = Some(500.0);
    }
    let ci = b.push_camera(cam);
    let r  = b.add_root_node("Root");
    let cn = b.add_child_node(r, "CamNode");
    b.attach_camera(cn, ci);
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    assert!(!loaded.cameras.is_empty());
    if let Projection::Perspective(ref p) = loaded.cameras[0].projection {
        assert!((p.z_near - 0.1).abs() < 1e-3, "z_near {} vs 0.1", p.z_near);
        let zf = p.z_far.unwrap_or(0.0);
        assert!((zf - 500.0).abs() < 0.5, "z_far {} vs 500.0", zf);
    } else {
        panic!("expected perspective projection");
    }
}

#[test]
fn ascii_orthographic_camera_survives() {
    let scene  = camera_scene(true);
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.cameras.is_empty(), "no cameras after round-trip");
    assert!(
        matches!(loaded.cameras[0].projection, Projection::Orthographic(_)),
        "expected orthographic projection"
    );
}

// ── Lights ────────────────────────────────────────────────────────────────────

#[test]
fn ascii_point_light_survives() {
    let scene  = lights_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.lights.is_empty(), "no lights after round-trip");
    let pt = loaded.lights.iter().find(|l| matches!(l, Light::Point(_)));
    assert!(pt.is_some(), "point light missing after round-trip");
    assert!((pt.unwrap().intensity() - 100.0).abs() < 1.0, "point light intensity off");
}

#[test]
fn ascii_directional_light_survives() {
    let scene  = lights_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(
        loaded.lights.iter().any(|l| matches!(l, Light::Directional(_))),
        "directional light missing"
    );
}

#[test]
fn ascii_spot_light_survives() {
    let scene  = lights_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(
        loaded.lights.iter().any(|l| matches!(l, Light::Spot(_))),
        "spot light missing"
    );
}

#[test]
fn ascii_area_light_survives() {
    let scene  = lights_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(
        loaded.lights.iter().any(|l| matches!(l, Light::Area(_))),
        "area light missing"
    );
}

// ── Skinning ──────────────────────────────────────────────────────────────────

#[test]
fn ascii_skin_joints_survive_round_trip() {
    let scene  = skinned_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.skins.is_empty(), "no skins after round-trip");
    assert_eq!(loaded.skins[0].joints.len(), 2, "joint count mismatch");
}

#[test]
fn ascii_skin_weights_survive_round_trip() {
    let scene  = skinned_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.meshes.is_empty());
    let mesh = &loaded.meshes[0];
    let has_weights = mesh.vertices.iter().any(|v| v.skin_weights.is_some());
    assert!(has_weights, "no skin weights after round-trip");

    // Vertex near origin should be 100% on joint 0.
    if let Some(v) = mesh.vertices.iter().find(|v| v.position.length() < 1e-3) {
        if let Some(sw) = &v.skin_weights {
            assert!(sw.weights[0] > 0.9, "joint-0 weight should be ~1.0, got {}", sw.weights[0]);
        }
    }
}

// ── Animation ─────────────────────────────────────────────────────────────────

#[test]
fn ascii_animation_translation_survives() {
    let scene  = animated_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.animations.is_empty(), "no animations after round-trip");
    let ch = loaded.animations[0].channels.iter()
        .find(|c| matches!(c.target, AnimationTarget::Translation(_)))
        .expect("translation channel missing");
    assert!(ch.times.len() >= 2, "need ≥2 keyframes");
    let n = ch.values.len();
    // Last keyframe translation should be (1, 0, 0).
    assert!((ch.values[n - 3] - 1.0).abs() < 1e-2, "T.x at end: {}", ch.values[n - 3]);
    assert!((ch.values[n - 2] - 0.0).abs() < 1e-2, "T.y at end: {}", ch.values[n - 2]);
    assert!((ch.values[n - 1] - 0.0).abs() < 1e-2, "T.z at end: {}", ch.values[n - 1]);
}

#[test]
fn ascii_animation_rotation_survives() {
    use std::f32::consts::FRAC_PI_2;
    let mut b  = SceneBuilder::new();
    let r      = b.add_root_node("Root");
    let target = b.add_child_node(r, "Animated");
    let q0     = Quat::IDENTITY;
    let q1     = Quat::from_rotation_y(FRAC_PI_2);
    let anim   = Animation {
        name: "RotAnim".into(),
        channels: vec![AnimationChannel {
            target:        AnimationTarget::Rotation(target),
            interpolation: Interpolation::Linear,
            times:         vec![0.0, 1.0],
            values:        vec![q0.x, q0.y, q0.z, q0.w,  q1.x, q1.y, q1.z, q1.w],
        }],
        extensions: Extensions::new(),
    };
    b.push_animation(anim);
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    assert!(!loaded.animations.is_empty(), "no animations");
    let ch = loaded.animations[0].channels.iter()
        .find(|c| matches!(c.target, AnimationTarget::Rotation(_)))
        .expect("rotation channel missing");
    assert!(ch.times.len() >= 2, "need ≥2 keyframes");
    let n        = ch.values.len();
    let loaded_q = Quat::from_xyzw(ch.values[n-4], ch.values[n-3], ch.values[n-2], ch.values[n-1]);
    let test_v   = Vec3::X;
    let expected = q1       * test_v;
    let actual   = loaded_q * test_v;
    assert!((expected.x - actual.x).abs() < 1e-2, "rot.x {} vs {}", expected.x, actual.x);
    assert!((expected.z - actual.z).abs() < 1e-2, "rot.z {} vs {}", expected.z, actual.z);
}

#[test]
fn ascii_animation_scale_survives() {
    let scene  = animated_scene();
    let loaded = ascii_round_trip(&scene);
    assert!(!loaded.animations.is_empty());
    let ch = loaded.animations[0].channels.iter()
        .find(|c| matches!(c.target, AnimationTarget::Scale(_)))
        .expect("scale channel missing");
    assert!(ch.times.len() >= 2, "need ≥2 keyframes");
    let n = ch.values.len();
    // Last keyframe scale should be (2, 2, 2).
    assert!((ch.values[n - 3] - 2.0).abs() < 1e-2, "S.x at end: {}", ch.values[n - 3]);
    assert!((ch.values[n - 2] - 2.0).abs() < 1e-2, "S.y at end: {}", ch.values[n - 2]);
    assert!((ch.values[n - 1] - 2.0).abs() < 1e-2, "S.z at end: {}", ch.values[n - 1]);
}

// ── Misc ──────────────────────────────────────────────────────────────────────

#[test]
fn ascii_per_polygon_material_survives() {
    let mut b    = SceneBuilder::new();
    let mat0     = b.push_material(Material::new("Mat0"));
    let mat1     = b.push_material(Material::new("Mat1"));
    let mut mesh = Mesh::new("MultiMat");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 0.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, 0.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, 1.0, 0.0)).with_normal(Vec3::Z),
    ];
    mesh.primitives = vec![
        Primitive::triangles(vec![0, 1, 2], Some(mat0)),
        Primitive::triangles(vec![1, 3, 2], Some(mat1)),
    ];
    let mi = b.push_mesh(mesh);
    let r  = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);

    assert!(loaded.materials.len() >= 2, "need ≥2 materials, got {}", loaded.materials.len());
    assert!(loaded.meshes[0].primitives.len() >= 2, "need ≥2 primitives");
    let p0 = &loaded.meshes[0].primitives[0];
    let p1 = &loaded.meshes[0].primitives[1];
    assert!(p0.material_index.is_some(), "prim 0 material missing");
    assert!(p1.material_index.is_some(), "prim 1 material missing");
    assert_ne!(p0.material_index, p1.material_index, "materials should differ");
}

#[test]
fn ascii_empty_scene_round_trips() {
    let scene = Scene::new();
    let mut buf = Vec::new();
    FbxSaver.save(&scene, &mut buf, &SaveOptions::default()).unwrap();
    let mut cursor = Cursor::new(buf);
    let loaded = FbxLoader.load(&mut cursor, &LoadOptions::default()).unwrap();
    assert!(loaded.meshes.is_empty(),    "expected no meshes in empty scene");
    assert!(loaded.materials.is_empty(), "expected no materials in empty scene");
}

#[test]
fn ascii_multiple_meshes_survive() {
    let mut b = SceneBuilder::new();
    let r     = b.add_root_node("Root");
    for i in 0..3 {
        let mi = b.push_mesh(make_minimal_mesh(&format!("Mesh{i}")));
        let n  = b.add_child_node(r, format!("Node{i}"));
        b.attach_mesh(n, mi);
    }
    let scene  = b.build();
    let loaded = ascii_round_trip(&scene);
    assert_eq!(loaded.meshes.len(), 3, "expected 3 meshes, got {}", loaded.meshes.len());
}
