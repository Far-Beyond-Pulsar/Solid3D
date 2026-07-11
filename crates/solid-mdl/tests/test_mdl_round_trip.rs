//! Round-trip tests: Scene → MDL → Scene.

mod common;

use common::*;
use solid_rs::prelude::*;

#[test]
fn round_trip_positions_survive() {
    let original = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(2.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 3.0, 0.0),
    );
    let loaded = round_trip(&original);
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

    // Due to u8 quantisation, allow a tolerance of 1/255th of the range
    let eps = 2.0 / 255.0; // slightly more than one quantisation step
    for p in &orig_positions {
        assert!(
            load_positions.iter().any(|lp| (*lp - *p).length() < eps),
            "position {p:?} not found after round-trip (eps={eps})"
        );
    }
}

#[test]
fn round_trip_triangle_count() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let loaded = round_trip(&scene);
    assert_eq!(
        total_triangle_count(&loaded),
        total_triangle_count(&scene),
        "triangle count must survive round-trip"
    );
}

#[test]
fn round_trip_normals_present() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let loaded = round_trip(&scene);
    for v in &loaded.meshes[0].vertices {
        assert!(
            v.normal.is_some(),
            "every vertex should have a normal after round-trip"
        );
    }
}

#[test]
fn round_trip_vertex_count_unchanged() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let loaded = round_trip(&scene);
    assert_eq!(
        loaded.meshes[0].vertices.len(),
        3,
        "vertex count should be 3 after round-trip"
    );
}

#[test]
fn round_trip_identity_quantisation() {
    // Test with positions that map to exact byte values
    let p0 = glam::Vec3::new(0.0, 0.0, 0.0);
    let p1 = glam::Vec3::new(255.0, 0.0, 0.0);
    let p2 = glam::Vec3::new(0.0, 255.0, 0.0);
    let scene = triangle_scene(p0, p1, p2);
    let loaded = round_trip(&scene);

    let positions: Vec<_> = loaded.meshes[0]
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    assert!(positions.contains(&p0));
    assert!(positions.contains(&p1));
    assert!(positions.contains(&p2));
}

#[test]
fn round_trip_multi_mesh() {
    let mut b = SceneBuilder::named("MultiMesh");
    let mut m1 = Mesh::new("A");
    m1.vertices = vec![
        Vertex::new(glam::Vec3::new(0.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(5.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(0.0, 5.0, 0.0)),
    ];
    m1.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];

    let mut m2 = Mesh::new("B");
    m2.vertices = vec![
        Vertex::new(glam::Vec3::new(10.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(15.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(10.0, 5.0, 0.0)),
    ];
    m2.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];

    let mi1 = b.push_mesh(m1);
    let mi2 = b.push_mesh(m2);
    let r1 = b.add_root_node("R1");
    let r2 = b.add_root_node("R2");
    b.attach_mesh(r1, mi1);
    b.attach_mesh(r2, mi2);
    let scene = b.build();

    let original_total = total_triangle_count(&scene);
    assert_eq!(original_total, 2);

    let loaded = round_trip(&scene);
    assert_eq!(
        total_triangle_count(&loaded),
        2,
        "both triangles must survive round-trip"
    );
}
