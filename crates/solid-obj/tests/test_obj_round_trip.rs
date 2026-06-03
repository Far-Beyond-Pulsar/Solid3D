mod common;
use common::*;
use glam::{Vec3, Vec4};
use solid_rs::prelude::*;
use solid_rs::scene::AlphaMode;

// ── Geometry round-trips ──────────────────────────────────────────────────────

#[test]
fn round_trip_positions() {
    let original = triangle_scene();
    let loaded = obj_round_trip(&original);

    let orig_positions: Vec<_> = original.meshes[0]
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    let load_positions: Vec<_> = loaded.meshes[0]
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();

    assert_eq!(orig_positions.len(), load_positions.len());
    for op in &orig_positions {
        assert!(
            load_positions.iter().any(|lp| (*lp - *op).length() < 1e-4),
            "position {:?} not found after round-trip",
            op
        );
    }
}

#[test]
fn round_trip_normals() {
    let original = triangle_scene();
    let loaded = obj_round_trip(&original);

    let loaded_verts = &loaded.meshes[0].vertices;
    assert!(
        loaded_verts.iter().all(|v| v.normal.is_some()),
        "normals should survive the round-trip"
    );
    for v in loaded_verts {
        let n = v.normal.unwrap();
        assert!(
            n.length() > 0.9,
            "normal should be (approx) unit length: {:?}",
            n
        );
    }
}

#[test]
fn round_trip_uvs() {
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    assert!(
        loaded.meshes[0].vertices.iter().all(|v| v.uvs[0].is_some()),
        "UV coordinates should survive the round-trip"
    );
}

// ── Material round-trips ──────────────────────────────────────────────────────

#[test]
fn round_trip_material_name() {
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    assert!(
        !loaded.materials.is_empty(),
        "materials should be present after round-trip"
    );
    assert_eq!(loaded.materials[0].name, "PBRMat");
}

#[test]
fn round_trip_diffuse_color() {
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    let c = loaded.materials[0].base_color_factor;
    assert!((c.x - 0.8).abs() < 1e-4, "Kd.r mismatch: {}", c.x);
    assert!((c.y - 0.4).abs() < 1e-4, "Kd.g mismatch: {}", c.y);
    assert!((c.z - 0.2).abs() < 1e-4, "Kd.b mismatch: {}", c.z);
}

#[test]
fn round_trip_emissive_color() {
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    let e = loaded.materials[0].emissive_factor;
    assert!((e.x - 0.1).abs() < 1e-4, "Ke.r mismatch: {}", e.x);
    assert!((e.y - 0.2).abs() < 1e-4, "Ke.g mismatch: {}", e.y);
    assert!((e.z - 0.3).abs() < 1e-4, "Ke.b mismatch: {}", e.z);
}

#[test]
fn round_trip_roughness() {
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    let r = loaded.materials[0].roughness_factor;
    assert!((r - 0.7).abs() < 1e-4, "roughness_factor mismatch: {}", r);
}

#[test]
fn round_trip_metallic() {
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    let m = loaded.materials[0].metallic_factor;
    assert!((m - 0.3).abs() < 1e-4, "metallic_factor mismatch: {}", m);
}

#[test]
fn round_trip_alpha_blend() {
    let mut b = SceneBuilder::new();
    let mut mat = Material::new("BlendMat");
    mat.base_color_factor = Vec4::new(1.0, 1.0, 1.0, 0.5);
    mat.alpha_mode = AlphaMode::Blend;
    let mi_m = b.push_material(mat);
    let mut mesh = Mesh::new("M");
    mesh.vertices = vec![
        Vertex::new(Vec3::ZERO),
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi_m))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("M");
    b.attach_mesh(r, mi);
    let original = b.build();

    let loaded = obj_round_trip(&original);
    let alpha = loaded.materials[0].base_color_factor.w;
    assert!(
        (alpha - 0.5).abs() < 1e-4,
        "blend alpha mismatch: {}",
        alpha
    );
    assert_eq!(loaded.materials[0].alpha_mode, AlphaMode::Blend);
}

#[test]
fn round_trip_alpha_mask() {
    // OBJ doesn't have a native Mask concept; it converts to Blend on load.
    // We verify the dissolve value (1 - alpha_cutoff) is preserved.
    let mut b = SceneBuilder::new();
    let mut mat = Material::new("MaskMat");
    mat.alpha_mode = AlphaMode::Mask;
    mat.alpha_cutoff = 0.3;
    let mi_m = b.push_material(mat);
    let mut mesh = Mesh::new("M");
    mesh.vertices = vec![
        Vertex::new(Vec3::ZERO),
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi_m))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("M");
    b.attach_mesh(r, mi);
    let original = b.build();

    let loaded = obj_round_trip(&original);
    // Saver writes "d 0.7000" (= 1.0 - 0.3), loader sets alpha = 0.7 → Blend
    let alpha = loaded.materials[0].base_color_factor.w;
    assert!(
        (alpha - 0.7).abs() < 1e-4,
        "mask dissolve mismatch: {}",
        alpha
    );
    // alpha < 1.0 → Blend after reload
    assert_eq!(loaded.materials[0].alpha_mode, AlphaMode::Blend);
}

// ── Texture round-trips ───────────────────────────────────────────────────────

#[test]
fn round_trip_diffuse_texture_uri() {
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    let mat = &loaded.materials[0];
    assert!(
        mat.base_color_texture.is_some(),
        "base_color_texture should survive round-trip"
    );
    let tex_idx = mat.base_color_texture.as_ref().unwrap().texture_index;
    assert_eq!(image_uri(&loaded, tex_idx), Some("diffuse.png"));
}

// ── Multi-mesh round-trips ────────────────────────────────────────────────────

#[test]
fn round_trip_multiple_meshes_count() {
    let original = multi_mesh_scene();
    let loaded = obj_round_trip(&original);
    assert_eq!(
        loaded.meshes.len(),
        2,
        "both meshes should survive the round-trip"
    );
}

#[test]
fn round_trip_mesh_names() {
    let original = multi_mesh_scene();
    let loaded = obj_round_trip(&original);

    // The saver writes group names from node names; loader restores them as mesh names.
    let names: Vec<&str> = loaded.meshes.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"MeshA"), "MeshA not found in {:?}", names);
    assert!(names.contains(&"MeshB"), "MeshB not found in {:?}", names);
}

// ── PBR extensions round-trip ─────────────────────────────────────────────────

#[test]
fn round_trip_pbr_extensions() {
    // Verify Pr and Pm survive the round-trip as first-class roughness/metallic.
    let original = material_scene();
    let loaded = obj_round_trip(&original);

    let mat = &loaded.materials[0];
    assert!(
        (mat.roughness_factor - 0.7).abs() < 1e-4,
        "Pr round-trip failed: {}",
        mat.roughness_factor
    );
    assert!(
        (mat.metallic_factor - 0.3).abs() < 1e-4,
        "Pm round-trip failed: {}",
        mat.metallic_factor
    );
}

// ── Empty scene ───────────────────────────────────────────────────────────────

#[test]
fn round_trip_empty_scene() {
    let original = Scene::new();
    let loaded = obj_round_trip(&original);
    assert_eq!(
        loaded.meshes.len(),
        0,
        "empty scene should remain empty after round-trip"
    );
    assert_eq!(loaded.materials.len(), 0);
}
