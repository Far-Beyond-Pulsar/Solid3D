mod common;
use common::*;

use glam::Vec3;
use solid_ply::PlySaver;
use solid_rs::prelude::*;

const EPS: f32 = 1e-5;
const COLOR_EPS: f32 = 2.0 / 255.0;

// ── ASCII round-trips ─────────────────────────────────────────────────────────

#[test]
fn round_trip_ascii_positions() {
    let scene = triangle_scene();
    let rt = ascii_round_trip(&scene);
    let orig = &scene.meshes[0].vertices;
    let loaded = &rt.meshes[0].vertices;
    assert_eq!(loaded.len(), orig.len());
    for (o, l) in orig.iter().zip(loaded.iter()) {
        assert!((o.position.x - l.position.x).abs() < EPS, "x mismatch");
        assert!((o.position.y - l.position.y).abs() < EPS, "y mismatch");
        assert!((o.position.z - l.position.z).abs() < EPS, "z mismatch");
    }
}

#[test]
fn round_trip_ascii_normals() {
    let scene = normal_scene();
    let rt = ascii_round_trip(&scene);
    let loaded = &rt.meshes[0].vertices;
    for v in loaded {
        let n = v.normal.expect("normal should survive ASCII round-trip");
        assert!((n.x - 0.0).abs() < EPS);
        assert!((n.y - 0.0).abs() < EPS);
        assert!((n.z - 1.0).abs() < EPS);
    }
}

#[test]
fn round_trip_ascii_colors() {
    let scene = colored_vertex_scene();
    let rt = ascii_round_trip(&scene);
    let orig_c = scene.meshes[0].vertices[0].colors[0].unwrap();
    let rt_c = rt.meshes[0].vertices[0].colors[0].expect("color should survive");
    assert!((rt_c.x - orig_c.x).abs() < COLOR_EPS, "red mismatch");
    assert!((rt_c.y - orig_c.y).abs() < COLOR_EPS, "green mismatch");
    assert!((rt_c.z - orig_c.z).abs() < COLOR_EPS, "blue mismatch");
    assert!((rt_c.w - orig_c.w).abs() < COLOR_EPS, "alpha mismatch");
}

#[test]
fn round_trip_ascii_uvs_channel_0() {
    let scene = uv_scene();
    let rt = ascii_round_trip(&scene);
    let orig_uv = scene.meshes[0].vertices[0].uvs[0].unwrap();
    let rt_uv = rt.meshes[0].vertices[0].uvs[0].expect("uvs[0] should survive");
    assert!((rt_uv.x - orig_uv.x).abs() < EPS, "u mismatch");
    assert!((rt_uv.y - orig_uv.y).abs() < EPS, "v mismatch");
}

#[test]
fn round_trip_ascii_uvs_channel_1() {
    let scene = multi_uv_scene();
    let rt = ascii_round_trip(&scene);
    let orig_uv1 = scene.meshes[0].vertices[0].uvs[1].unwrap();
    let rt_uv1 = rt.meshes[0].vertices[0].uvs[1].expect("uvs[1] should survive");
    assert!((rt_uv1.x - orig_uv1.x).abs() < EPS, "u1 mismatch");
    assert!((rt_uv1.y - orig_uv1.y).abs() < EPS, "v1 mismatch");
}

#[test]
fn round_trip_ascii_tangents() {
    // Tangents are emitted by the saver but not loaded back (loader doesn't handle them).
    // This test verifies positions survive even when tangent properties are present.
    let scene = tangent_scene();
    let rt = ascii_round_trip(&scene);
    let orig = &scene.meshes[0].vertices;
    let loaded = &rt.meshes[0].vertices;
    assert_eq!(loaded.len(), orig.len());
    for (o, l) in orig.iter().zip(loaded.iter()) {
        assert!(
            (o.position.x - l.position.x).abs() < EPS,
            "x mismatch with tangent props"
        );
        assert!(
            (o.position.y - l.position.y).abs() < EPS,
            "y mismatch with tangent props"
        );
        assert!(
            (o.position.z - l.position.z).abs() < EPS,
            "z mismatch with tangent props"
        );
    }
}

#[test]
fn round_trip_ascii_index_count() {
    let scene = triangle_scene();
    let rt = ascii_round_trip(&scene);
    let orig_prim = &scene.meshes[0].primitives[0];
    let rt_prim = &rt.meshes[0].primitives[0];
    assert_eq!(
        orig_prim.indices.len(),
        rt_prim.indices.len(),
        "index count mismatch"
    );
}

// ── Binary LE round-trips ─────────────────────────────────────────────────────

#[test]
fn round_trip_binary_le_positions() {
    let scene = triangle_scene();
    let rt = binary_le_round_trip(&scene);
    let orig = &scene.meshes[0].vertices;
    let loaded = &rt.meshes[0].vertices;
    assert_eq!(loaded.len(), orig.len());
    for (o, l) in orig.iter().zip(loaded.iter()) {
        assert!((o.position.x - l.position.x).abs() < EPS);
        assert!((o.position.y - l.position.y).abs() < EPS);
        assert!((o.position.z - l.position.z).abs() < EPS);
    }
}

#[test]
fn round_trip_binary_le_normals() {
    let scene = normal_scene();
    let rt = binary_le_round_trip(&scene);
    for v in &rt.meshes[0].vertices {
        let n = v
            .normal
            .expect("normal should survive binary LE round-trip");
        assert!((n.z - 1.0).abs() < EPS);
    }
}

#[test]
fn round_trip_binary_le_colors() {
    let scene = colored_vertex_scene();
    let rt = binary_le_round_trip(&scene);
    let orig_c = scene.meshes[0].vertices[0].colors[0].unwrap();
    let rt_c = rt.meshes[0].vertices[0].colors[0].expect("color should survive LE round-trip");
    assert!((rt_c.x - orig_c.x).abs() < COLOR_EPS);
    assert!((rt_c.y - orig_c.y).abs() < COLOR_EPS);
    assert!((rt_c.z - orig_c.z).abs() < COLOR_EPS);
    assert!((rt_c.w - orig_c.w).abs() < COLOR_EPS);
}

// ── Binary BE round-trips ─────────────────────────────────────────────────────

#[test]
fn round_trip_binary_be_positions() {
    let scene = triangle_scene();
    let rt = binary_be_round_trip(&scene);
    let orig = &scene.meshes[0].vertices;
    let loaded = &rt.meshes[0].vertices;
    assert_eq!(loaded.len(), orig.len());
    for (o, l) in orig.iter().zip(loaded.iter()) {
        assert!((o.position.x - l.position.x).abs() < EPS);
        assert!((o.position.y - l.position.y).abs() < EPS);
        assert!((o.position.z - l.position.z).abs() < EPS);
    }
}

#[test]
fn round_trip_binary_be_normals() {
    let scene = normal_scene();
    let rt = binary_be_round_trip(&scene);
    for v in &rt.meshes[0].vertices {
        let n = v
            .normal
            .expect("normal should survive binary BE round-trip");
        assert!((n.z - 1.0).abs() < EPS);
    }
}

// ── Double precision round-trip ───────────────────────────────────────────────

#[test]
fn round_trip_double_precision_positions() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    PlySaver::save_with_precision(&scene, &mut buf, true).unwrap();

    use solid_ply::PlyLoader;
    use solid_rs::prelude::*;
    use std::io::Cursor;
    let rt = PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap();

    let orig = &scene.meshes[0].vertices;
    let loaded = &rt.meshes[0].vertices;
    assert_eq!(loaded.len(), orig.len());
    for (o, l) in orig.iter().zip(loaded.iter()) {
        assert!(
            (o.position.x - l.position.x).abs() < EPS,
            "x mismatch in double precision"
        );
        assert!(
            (o.position.y - l.position.y).abs() < EPS,
            "y mismatch in double precision"
        );
        assert!(
            (o.position.z - l.position.z).abs() < EPS,
            "z mismatch in double precision"
        );
    }
}

// ── Point cloud round-trip ────────────────────────────────────────────────────

#[test]
fn round_trip_point_cloud_no_faces() {
    let scene = point_cloud_scene();
    let rt = ascii_round_trip(&scene);
    let loaded_mesh = &rt.meshes[0];
    assert_eq!(
        loaded_mesh.vertices.len(),
        5,
        "all 5 cloud points should survive"
    );
    let has_triangles = loaded_mesh
        .primitives
        .iter()
        .any(|p| p.topology == Topology::TriangleList);
    assert!(
        !has_triangles,
        "reloaded point cloud must not have triangle faces"
    );
}

// ── Multi-mesh round-trip ─────────────────────────────────────────────────────

#[test]
fn round_trip_multi_mesh_vertex_count() {
    let mut b = SceneBuilder::named("Two Meshes");
    let mut m1 = Mesh::new("A");
    m1.vertices = vec![
        Vertex::new(Vec3::new(0.0, 0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, 0.0, 0.0)),
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
    ];
    m1.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mut m2 = Mesh::new("B");
    m2.vertices = vec![
        Vertex::new(Vec3::new(2.0, 0.0, 0.0)),
        Vertex::new(Vec3::new(3.0, 0.0, 0.0)),
        Vertex::new(Vec3::new(2.0, 1.0, 0.0)),
    ];
    m2.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let i1 = b.push_mesh(m1);
    let i2 = b.push_mesh(m2);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, i1);
    b.attach_mesh(r, i2);
    let scene = b.build();

    let rt = ascii_round_trip(&scene);
    assert_eq!(
        rt.meshes[0].vertices.len(),
        6,
        "two 3-vert meshes merged into one PLY and reloaded must give 6 vertices"
    );
}
