//! Round-trip integration tests: save → reload, verify data is preserved.

mod common;

use common::*;
use glam::{Vec3, Vec4};
use solid_rs::prelude::*;
use solid_rs::scene::{AlphaMode, Projection};
use solid_rs::geometry::{Primitive, Vertex};
use solid_rs::builder::SceneBuilder;

// ── Vertex geometry ───────────────────────────────────────────────────────────

#[test]
fn round_trip_positions() {
    let original = triangle_scene();
    let loaded = gltf_round_trip(&original);
    let orig_pos: Vec<Vec3> = original.meshes[0].vertices.iter().map(|v| v.position).collect();
    let load_pos: Vec<Vec3> = loaded.meshes[0].vertices.iter().map(|v| v.position).collect();
    assert_eq!(orig_pos.len(), load_pos.len());
    for (o, l) in orig_pos.iter().zip(load_pos.iter()) {
        assert!((o.x - l.x).abs() < 1e-5 && (o.y - l.y).abs() < 1e-5 && (o.z - l.z).abs() < 1e-5,
            "position mismatch: {o:?} vs {l:?}");
    }
}

#[test]
fn round_trip_normals() {
    let original = triangle_scene();
    let loaded = gltf_round_trip(&original);
    let verts = &loaded.meshes[0].vertices;
    assert!(verts.iter().all(|v| v.normal.is_some()), "normals should survive round-trip");
    for v in verts {
        let n = v.normal.unwrap();
        assert!((n - Vec3::Z).length() < 1e-5, "normal should be Vec3::Z, got {n:?}");
    }
}

#[test]
fn round_trip_uvs() {
    let original = pbr_material_scene();
    let loaded = gltf_round_trip(&original);
    let orig_uvs: Vec<_> = original.meshes[0].vertices.iter().map(|v| v.uvs[0].unwrap()).collect();
    let load_uvs: Vec<_> = loaded.meshes[0].vertices.iter().map(|v| v.uvs[0].unwrap()).collect();
    for (o, l) in orig_uvs.iter().zip(load_uvs.iter()) {
        assert!((o.x - l.x).abs() < 1e-5 && (o.y - l.y).abs() < 1e-5,
            "UV mismatch: {o:?} vs {l:?}");
    }
}

#[test]
fn round_trip_tangents() {
    let mut b = SceneBuilder::named("Tangent Round-Trip");
    let mut mesh = solid_rs::scene::Mesh::new("TM");
    mesh.vertices = vec![
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
        Vertex::new(Vec3::Z),
    ];
    let tangent_val = glam::Vec4::new(1.0, 0.0, 0.0, 1.0);
    for v in &mut mesh.vertices {
        v.tangent = Some(tangent_val);
    }
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("R");
    b.attach_mesh(r, mi);
    let original = b.build();

    let loaded = gltf_round_trip(&original);
    for v in &loaded.meshes[0].vertices {
        let t = v.tangent.expect("tangent should survive round-trip");
        assert!((t.x - 1.0).abs() < 1e-5, "tangent.x should be 1.0");
        assert!((t.w - 1.0).abs() < 1e-5, "tangent.w should be 1.0");
    }
}

#[test]
fn round_trip_vertex_colors() {
    let original = pbr_material_scene();
    let loaded = gltf_round_trip(&original);
    let orig_colors: Vec<Vec4> = original.meshes[0].vertices.iter()
        .map(|v| v.colors[0].unwrap()).collect();
    let load_colors: Vec<Vec4> = loaded.meshes[0].vertices.iter()
        .map(|v| v.colors[0].unwrap()).collect();
    for (o, l) in orig_colors.iter().zip(load_colors.iter()) {
        assert!((o.x - l.x).abs() < 1e-5, "color.r mismatch: {o:?} vs {l:?}");
    }
}

// ── Material properties ───────────────────────────────────────────────────────

#[test]
fn round_trip_material_base_color() {
    let original = pbr_material_scene();
    let loaded = gltf_round_trip(&original);
    let o = original.materials[0].base_color_factor;
    let l = loaded.materials[0].base_color_factor;
    assert!((o.x - l.x).abs() < 1e-5, "base_color R mismatch");
    assert!((o.y - l.y).abs() < 1e-5, "base_color G mismatch");
}

#[test]
fn round_trip_material_roughness() {
    let original = pbr_material_scene();
    let loaded = gltf_round_trip(&original);
    let o = original.materials[0].roughness_factor;
    let l = loaded.materials[0].roughness_factor;
    assert!((o - l).abs() < 1e-5, "roughness_factor mismatch: {o} vs {l}");
}

#[test]
fn round_trip_material_metallic() {
    let original = pbr_material_scene();
    let loaded = gltf_round_trip(&original);
    let o = original.materials[0].metallic_factor;
    let l = loaded.materials[0].metallic_factor;
    assert!((o - l).abs() < 1e-5, "metallic_factor mismatch: {o} vs {l}");
}

#[test]
fn round_trip_material_emissive() {
    let original = pbr_material_scene();
    let loaded = gltf_round_trip(&original);
    let o = original.materials[0].emissive_factor;
    let l = loaded.materials[0].emissive_factor;
    assert!((o.x - l.x).abs() < 1e-5, "emissive R mismatch: {o:?} vs {l:?}");
}

#[test]
fn round_trip_material_specular_ior() {
    let original = pbr_specular_ior_scene();
    let loaded = gltf_round_trip(&original);
    let authored = &original.materials[0];
    let round_tripped = &loaded.materials[0];

    assert!((authored.specular_weight - round_tripped.specular_weight).abs() < 1e-5);
    assert!((authored.specular_color.x - round_tripped.specular_color.x).abs() < 1e-5);
    assert!((authored.specular_color.y - round_tripped.specular_color.y).abs() < 1e-5);
    assert!((authored.specular_color.z - round_tripped.specular_color.z).abs() < 1e-5);
    assert!((authored.ior - round_tripped.ior).abs() < 1e-5);
}

#[test]
fn round_trip_alpha_opaque() {
    let original = pbr_material_scene();
    let loaded = gltf_round_trip(&original);
    assert_eq!(loaded.materials[0].alpha_mode, AlphaMode::Opaque);
}

#[test]
fn round_trip_alpha_blend() {
    let mut b = SceneBuilder::named("Blend");
    let mut mat = solid_rs::scene::Material::new("BlendMat");
    mat.alpha_mode = AlphaMode::Blend;
    b.push_material(mat);
    b.add_root_node("R");
    let original = b.build();
    let loaded = gltf_round_trip(&original);
    assert_eq!(loaded.materials[0].alpha_mode, AlphaMode::Blend);
}

#[test]
fn round_trip_alpha_mask() {
    let mut b = SceneBuilder::named("Mask");
    let mut mat = solid_rs::scene::Material::new("MaskMat");
    mat.alpha_mode = AlphaMode::Mask;
    mat.alpha_cutoff = 0.3;
    b.push_material(mat);
    b.add_root_node("R");
    let original = b.build();
    let loaded = gltf_round_trip(&original);
    assert_eq!(loaded.materials[0].alpha_mode, AlphaMode::Mask);
    assert!((loaded.materials[0].alpha_cutoff - 0.3).abs() < 1e-5,
        "alpha_cutoff mismatch: {}", loaded.materials[0].alpha_cutoff);
}

// ── Cameras ───────────────────────────────────────────────────────────────────

#[test]
fn round_trip_perspective_camera_fov() {
    let original = camera_scene();
    let loaded = gltf_round_trip(&original);
    match &loaded.cameras[0].projection {
        Projection::Perspective(p) => {
            assert!((p.fov_y - 0.785398).abs() < 1e-4, "fov_y mismatch: {}", p.fov_y);
        }
        _ => panic!("expected perspective projection"),
    }
}

#[test]
fn round_trip_orthographic_camera_mag() {
    let original = camera_scene();
    let loaded = gltf_round_trip(&original);
    match &loaded.cameras[1].projection {
        Projection::Orthographic(o) => {
            assert!((o.x_mag - 5.0).abs() < 1e-5, "x_mag mismatch: {}", o.x_mag);
            assert!((o.y_mag - 5.0).abs() < 1e-5, "y_mag mismatch: {}", o.y_mag);
        }
        _ => panic!("expected orthographic projection"),
    }
}

// ── Node hierarchy ────────────────────────────────────────────────────────────

#[test]
fn round_trip_node_hierarchy() {
    let original = camera_scene();
    let loaded = gltf_round_trip(&original);
    let root = loaded.node(*loaded.roots.first().unwrap()).unwrap();
    assert_eq!(root.children.len(), 2, "root should preserve 2 children");
}

// ── Skin ─────────────────────────────────────────────────────────────────────

#[test]
fn round_trip_skin_joint_count() {
    let original = skinned_scene();
    let loaded = gltf_round_trip(&original);
    assert_eq!(loaded.skins[0].joints.len(), 2, "skin joint count should round-trip");
}

#[test]
fn round_trip_skin_ibp_matrices() {
    let original = skinned_scene();
    let loaded = gltf_round_trip(&original);
    let ibms = &loaded.skins[0].inverse_bind_matrices;
    assert_eq!(ibms.len(), 2);
    for m in ibms {
        // identity matrices should survive
        let _diff = (*m - glam::Mat4::IDENTITY).abs_diff_eq(glam::Mat4::IDENTITY, 1e-5);
        // Actually just check all columns are approximately right
        let cols = m.to_cols_array();
        let id = glam::Mat4::IDENTITY.to_cols_array();
        for (a, b) in cols.iter().zip(id.iter()) {
            assert!((a - b).abs() < 1e-5, "IBP matrix element mismatch");
        }
    }
}

// ── Animation ────────────────────────────────────────────────────────────────

#[test]
fn round_trip_animation_channel_count() {
    let original = animated_scene();
    let loaded = gltf_round_trip(&original);
    assert_eq!(loaded.animations[0].channels.len(), 2, "animation channel count should round-trip");
}

#[test]
fn round_trip_animation_times() {
    let original = animated_scene();
    let loaded = gltf_round_trip(&original);
    let times = &loaded.animations[0].channels[0].times;
    assert_eq!(times.len(), 2, "expected 2 keyframe times");
    assert!((times[0] - 0.0).abs() < 1e-5);
    assert!((times[1] - 1.0).abs() < 1e-5);
}

// ── Morph targets ─────────────────────────────────────────────────────────────

#[test]
fn round_trip_morph_target_count() {
    let original = morph_target_scene();
    let loaded = gltf_round_trip(&original);
    assert_eq!(loaded.meshes[0].morph_targets.len(), 2, "morph target count should round-trip");
}

#[test]
fn round_trip_morph_target_positions() {
    let original = morph_target_scene();
    let loaded = gltf_round_trip(&original);
    let deltas = &loaded.meshes[0].morph_targets[0].position_deltas;
    assert_eq!(deltas.len(), 3, "morph target should have 3 position deltas");
    for d in deltas {
        assert!((d.y - 0.1).abs() < 1e-5, "smile delta.y should be ~0.1, got {}", d.y);
    }
}

#[test]
fn round_trip_morph_weights() {
    let original = morph_target_scene();
    let loaded = gltf_round_trip(&original);
    let weights = &loaded.meshes[0].morph_weights;
    assert_eq!(weights.len(), 2, "morph weights count should round-trip");
    assert!((weights[0] - 0.0).abs() < 1e-5, "weight[0] should be 0.0");
    assert!((weights[1] - 0.5).abs() < 1e-5, "weight[1] should be 0.5");
}

// ── KHR_lights_punctual ───────────────────────────────────────────────────────

#[test]
fn round_trip_khr_lights_point() {
    use solid_rs::scene::Light;
    let original = lights_scene();
    let loaded = gltf_round_trip(&original);
    let point_light = loaded.lights.iter().find(|l| matches!(l, Light::Point(_)));
    assert!(point_light.is_some(), "point light should survive round-trip");
    if let Some(Light::Point(pl)) = point_light {
        assert!((pl.base.intensity - 200.0).abs() < 1e-3, "intensity mismatch: {}", pl.base.intensity);
    }
}

#[test]
fn round_trip_khr_lights_directional() {
    use solid_rs::scene::Light;
    let original = lights_scene();
    let loaded = gltf_round_trip(&original);
    let dir_light = loaded.lights.iter().find(|l| matches!(l, Light::Directional(_)));
    assert!(dir_light.is_some(), "directional light should survive round-trip");
}

// ── GLB binary round-trip ─────────────────────────────────────────────────────

#[test]
fn round_trip_glb_positions() {
    let original = triangle_scene();
    let loaded = glb_round_trip(&original);
    let orig_pos: Vec<Vec3> = original.meshes[0].vertices.iter().map(|v| v.position).collect();
    let load_pos: Vec<Vec3> = loaded.meshes[0].vertices.iter().map(|v| v.position).collect();
    assert_eq!(orig_pos.len(), load_pos.len());
    for (o, l) in orig_pos.iter().zip(load_pos.iter()) {
        assert!((o.x - l.x).abs() < 1e-5 && (o.y - l.y).abs() < 1e-5 && (o.z - l.z).abs() < 1e-5,
            "GLB position mismatch: {o:?} vs {l:?}");
    }
}

#[test]
fn round_trip_glb_material_count() {
    let original = pbr_material_scene();
    let loaded = glb_round_trip(&original);
    assert_eq!(loaded.materials.len(), original.materials.len(),
        "material count should round-trip through GLB");
}
