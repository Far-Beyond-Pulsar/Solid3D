//! Integration tests: USDA round-trip (save → reload → verify).

mod common;
use common::*;

use glam::Vec3;

// ── Geometry ──────────────────────────────────────────────────────────────────

#[test]
fn round_trip_vertex_count() {
    let original = triangle_scene();
    let loaded = usda_round_trip(&original);
    assert_eq!(
        original.meshes[0].vertices.len(),
        loaded.meshes[0].vertices.len(),
        "vertex count should survive round-trip",
    );
}

#[test]
fn round_trip_positions() {
    let original = triangle_scene();
    let loaded = usda_round_trip(&original);
    let orig_pos: Vec<Vec3> = original.meshes[0]
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    let load_pos: Vec<Vec3> = loaded.meshes[0]
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    for (o, l) in orig_pos.iter().zip(load_pos.iter()) {
        assert!(
            (o.x - l.x).abs() < 1e-4 && (o.y - l.y).abs() < 1e-4 && (o.z - l.z).abs() < 1e-4,
            "position mismatch: {o:?} vs {l:?}",
        );
    }
}

#[test]
fn round_trip_normals() {
    let original = triangle_scene();
    let loaded = usda_round_trip(&original);
    assert!(
        loaded.meshes[0].vertices.iter().all(|v| v.normal.is_some()),
        "normals should survive round-trip",
    );
    for v in &loaded.meshes[0].vertices {
        let n = v.normal.unwrap();
        assert!(
            (n - Vec3::Z).length() < 1e-4,
            "normal should be Vec3::Z, got {n:?}",
        );
    }
}

#[test]
fn round_trip_triangle_indices() {
    let original = triangle_scene();
    let loaded = usda_round_trip(&original);
    let orig_idx_count = original.meshes[0].primitives[0].indices.len();
    let load_idx_count = loaded.meshes[0].primitives[0].indices.len();
    assert_eq!(
        orig_idx_count, load_idx_count,
        "index count should round-trip"
    );
}

// ── Materials ─────────────────────────────────────────────────────────────────

#[test]
fn round_trip_material_count() {
    let original = material_scene();
    let loaded = usda_round_trip(&original);
    assert_eq!(
        original.materials.len(),
        loaded.materials.len(),
        "material count should round-trip",
    );
}

#[test]
fn round_trip_base_color() {
    let original = material_scene();
    let loaded = usda_round_trip(&original);
    let o = original.materials[0].base_color_factor;
    let l = loaded.materials[0].base_color_factor;
    assert!(
        (o.x - l.x).abs() < 1e-4,
        "base_color.r mismatch: {} vs {}",
        o.x,
        l.x
    );
    assert!(
        (o.y - l.y).abs() < 1e-4,
        "base_color.g mismatch: {} vs {}",
        o.y,
        l.y
    );
}

#[test]
fn round_trip_roughness() {
    let original = material_scene();
    let loaded = usda_round_trip(&original);
    let o = original.materials[0].roughness_factor;
    let l = loaded.materials[0].roughness_factor;
    assert!((o - l).abs() < 1e-4, "roughness mismatch: {o} vs {l}");
}

#[test]
fn round_trip_metallic() {
    let original = material_scene();
    let loaded = usda_round_trip(&original);
    let o = original.materials[0].metallic_factor;
    let l = loaded.materials[0].metallic_factor;
    assert!((o - l).abs() < 1e-4, "metallic mismatch: {o} vs {l}");
}

// ── Scene graph ───────────────────────────────────────────────────────────────

#[test]
fn round_trip_mesh_count() {
    let original = triangle_scene();
    let loaded = usda_round_trip(&original);
    assert_eq!(original.meshes.len(), loaded.meshes.len(), "mesh count");
}

#[test]
fn round_trip_node_count() {
    let original = hierarchy_scene();
    let loaded = usda_round_trip(&original);
    // We expect at least the root Xform + Parent + Child
    assert!(
        loaded.nodes.len() >= original.nodes.len(),
        "loaded nodes ({}) should be >= original ({})",
        loaded.nodes.len(),
        original.nodes.len(),
    );
}

// ── USDA output sanity ────────────────────────────────────────────────────────

#[test]
fn output_has_usda_header() {
    let scene = triangle_scene();
    let usda = usda_to_string(&scene);
    assert!(
        usda.starts_with("#usda 1.0"),
        "output must begin with #usda 1.0"
    );
}

#[test]
fn output_has_upaxis() {
    let scene = triangle_scene();
    let usda = usda_to_string(&scene);
    assert!(usda.contains("upAxis"), "output must contain upAxis");
}

#[test]
fn output_has_def_mesh() {
    let scene = triangle_scene();
    let usda = usda_to_string(&scene);
    assert!(
        usda.contains("def Mesh"),
        "output must contain a def Mesh prim"
    );
}

#[test]
fn output_has_points_attr() {
    let scene = triangle_scene();
    let usda = usda_to_string(&scene);
    assert!(
        usda.contains("point3f[] points"),
        "output must contain point3f[] points"
    );
}

#[test]
fn output_has_material_prim() {
    let scene = material_scene();
    let usda = usda_to_string(&scene);
    assert!(
        usda.contains("def Material"),
        "output must contain a def Material prim"
    );
}
