//! Round-trip tests: Scene → binary/ASCII STL bytes → Scene.

mod common;

use common::*;
use solid_rs::prelude::*;

// ── Binary round-trip ─────────────────────────────────────────────────────────

#[test]
fn round_trip_binary_positions() {
    let original = triangle_scene(
        glam::Vec3::new(1.0, 2.0, 3.0),
        glam::Vec3::new(4.0, 5.0, 6.0),
        glam::Vec3::new(7.0, 8.0, 9.0),
    );
    let loaded = binary_round_trip(&original);
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
    for p in &orig_positions {
        assert!(
            load_positions.iter().any(|lp| (*lp - *p).length() < 1e-5),
            "position {p:?} not found in loaded scene"
        );
    }
}

#[test]
fn round_trip_binary_triangle_count() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        total_triangle_count(&loaded),
        total_triangle_count(&scene),
        "triangle count must survive binary round-trip"
    );
}

#[test]
fn round_trip_binary_normal_directions() {
    // After a round-trip the loader recomputes smooth normals; they should
    // be present and face the correct half-space.
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let loaded = binary_round_trip(&scene);
    for v in &loaded.meshes[0].vertices {
        let n = v
            .normal
            .expect("every vertex should have a smooth normal after loading");
        // XY-plane triangle → normal must point in +Z direction
        assert!(n.z > 0.5, "normal should point toward +Z, got {n:?}");
    }
}

// ── ASCII round-trip ──────────────────────────────────────────────────────────

#[test]
fn round_trip_ascii_positions() {
    let original = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.5, 0.0, 0.0),
        glam::Vec3::new(0.0, 2.5, 0.0),
    );
    let loaded = ascii_round_trip(&original);
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
    for p in &orig_positions {
        assert!(
            load_positions.iter().any(|lp| (*lp - *p).length() < 1e-4),
            "position {p:?} not found after ASCII round-trip"
        );
    }
}

#[test]
fn round_trip_ascii_triangle_count() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let loaded = ascii_round_trip(&scene);
    assert_eq!(
        total_triangle_count(&loaded),
        total_triangle_count(&scene),
        "triangle count must survive ASCII round-trip"
    );
}

// ── VisCAM color round-trip ───────────────────────────────────────────────────

/// 5-bit quantisation → maximum rounding error per channel.
const COLOR_EPS: f32 = 1.0 / 31.0 + 1e-4;

fn check_color_approx(loaded: &solid_rs::scene::Scene, expected: glam::Vec4) {
    let verts = &loaded.meshes[0].vertices;
    let found = verts
        .iter()
        .find(|v| v.colors[0].is_some())
        .expect("at least one vertex should carry a color");
    let c = found.colors[0].unwrap();
    assert!(
        (c.x - expected.x).abs() < COLOR_EPS,
        "R: expected {:.3}, got {:.3}",
        expected.x,
        c.x
    );
    assert!(
        (c.y - expected.y).abs() < COLOR_EPS,
        "G: expected {:.3}, got {:.3}",
        expected.y,
        c.y
    );
    assert!(
        (c.z - expected.z).abs() < COLOR_EPS,
        "B: expected {:.3}, got {:.3}",
        expected.z,
        c.z
    );
}

#[test]
fn round_trip_viscam_color_red() {
    let scene = colored_triangle_scene(glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
    let loaded = binary_round_trip(&scene);
    check_color_approx(&loaded, glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
}

#[test]
fn round_trip_viscam_color_blue() {
    let scene = colored_triangle_scene(glam::Vec4::new(0.0, 0.0, 1.0, 1.0));
    let loaded = binary_round_trip(&scene);
    check_color_approx(&loaded, glam::Vec4::new(0.0, 0.0, 1.0, 1.0));
}

#[test]
fn round_trip_viscam_color_green() {
    let scene = colored_triangle_scene(glam::Vec4::new(0.0, 1.0, 0.0, 1.0));
    let loaded = binary_round_trip(&scene);
    check_color_approx(&loaded, glam::Vec4::new(0.0, 1.0, 0.0, 1.0));
}

#[test]
fn round_trip_viscam_color_white() {
    let scene = colored_triangle_scene(glam::Vec4::new(1.0, 1.0, 1.0, 1.0));
    let loaded = binary_round_trip(&scene);
    check_color_approx(&loaded, glam::Vec4::new(1.0, 1.0, 1.0, 1.0));
}

// ── Smooth normals ────────────────────────────────────────────────────────────

#[test]
fn round_trip_smooth_normals_present() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let loaded = binary_round_trip(&scene);
    for v in &loaded.meshes[0].vertices {
        assert!(
            v.normal.is_some(),
            "smooth normals should be computed for every vertex after loading"
        );
    }
}

// ── Multi-mesh ────────────────────────────────────────────────────────────────

#[test]
fn round_trip_multi_mesh_triangle_count() {
    // Build a 2-mesh scene: 1 tri + 2 tris = 3 triangles total.
    let mut b = SceneBuilder::named("MultiMesh");

    let mut m1 = Mesh::new("A");
    m1.vertices = vec![
        Vertex::new(glam::Vec3::new(0.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(1.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(0.0, 1.0, 0.0)),
    ];
    m1.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];

    let mut m2 = Mesh::new("B");
    m2.vertices = vec![
        Vertex::new(glam::Vec3::new(5.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(6.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(5.0, 1.0, 0.0)),
        Vertex::new(glam::Vec3::new(6.0, 1.0, 0.0)),
    ];
    m2.primitives = vec![Primitive::triangles(vec![0, 1, 2, 1, 3, 2], None)];

    let mi1 = b.push_mesh(m1);
    let mi2 = b.push_mesh(m2);
    let r1 = b.add_root_node("R1");
    let r2 = b.add_root_node("R2");
    b.attach_mesh(r1, mi1);
    b.attach_mesh(r2, mi2);
    let scene = b.build();

    let original_total = total_triangle_count(&scene);
    assert_eq!(original_total, 3);

    // Binary STL flattens all meshes; reloading recreates a single mesh.
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        total_triangle_count(&loaded),
        3,
        "all 3 triangles must survive the binary round-trip"
    );
}

// ── Large mesh ────────────────────────────────────────────────────────────────

#[test]
fn round_trip_large_mesh() {
    // 100 non-overlapping triangles (all vertices unique → no dedup).
    let n = 100u32;
    let mut b = SceneBuilder::named("Large");
    let mut mesh = Mesh::new("LargeMesh");
    for i in 0..n {
        let x = i as f32 * 10.0; // spread far apart so no shared positions
        mesh.vertices
            .push(Vertex::new(glam::Vec3::new(x, 0.0, 0.0)));
        mesh.vertices
            .push(Vertex::new(glam::Vec3::new(x, 1.0, 0.0)));
        mesh.vertices
            .push(Vertex::new(glam::Vec3::new(x + 0.5, 0.5, 1.0)));
    }
    let indices: Vec<u32> = (0..n * 3).collect();
    mesh.primitives = vec![Primitive::triangles(indices, None)];
    let mi = b.push_mesh(mesh);
    let root = b.add_root_node("Root");
    b.attach_mesh(root, mi);
    let scene = b.build();

    let loaded = binary_round_trip(&scene);
    assert_eq!(
        total_triangle_count(&loaded),
        100,
        "100 triangles should survive the binary round-trip"
    );
    assert_eq!(
        loaded.meshes[0].vertices.len(),
        300,
        "300 unique vertices expected (no deduplication for non-overlapping tris)"
    );
}
