//! Regression tests for bugs filed against solid-obj:
//!   #19  Saver panics on an index buffer that is not a multiple of 3
//!   #20  Loader silently drops faces referencing out-of-range vertices
//!   #21  Saver panics on an out-of-range `material_index`

use solid_obj::{ObjLoader, ObjSaver};
use solid_rs::prelude::*;
use std::io::Cursor;

// ── #19  Saver index buffer must be a multiple of 3 ───────────────────────────

#[test]
fn saver_errors_on_non_triangle_index_buffer_not_panics() {
    let mut b = SceneBuilder::new();
    let mut mesh = Mesh::new("Bad");
    mesh.vertices = (0..4)
        .map(|i| Vertex::new(glam::Vec3::new(i as f32, 0.0, 0.0)))
        .collect();
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2, 3], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene = b.build();

    let mut buf = Vec::new();
    let err = ObjSaver.save(&scene, &mut buf, &SaveOptions::default()).unwrap_err();
    assert!(
        format!("{err:?}").contains("multiple of 3"),
        "expected multiple-of-3 error, got {err:?}"
    );
}

#[test]
fn saver_errors_on_out_of_range_face_index() {
    let mut b = SceneBuilder::new();
    let mut mesh = Mesh::new("OOB");
    mesh.vertices = vec![Vertex::new(glam::Vec3::ZERO); 3];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 99], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene = b.build();

    let mut buf = Vec::new();
    let err = ObjSaver.save(&scene, &mut buf, &SaveOptions::default()).unwrap_err();
    assert!(
        format!("{err:?}").contains("beyond"),
        "expected out-of-range vertex error, got {err:?}"
    );
}

// ── #20  Loader must error on out-of-range face references ────────────────────

fn load_obj(src: &str) -> Result<Scene, solid_rs::SolidError> {
    let mut cursor = Cursor::new(src.as_bytes().to_vec());
    ObjLoader.load(&mut cursor, &LoadOptions::default())
}

#[test]
fn loader_errors_on_out_of_range_face_vertex() {
    let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 9\n";
    let err = load_obj(obj).unwrap_err();
    assert!(
        format!("{err:?}").contains("out of range"),
        "expected out-of-range error, got {err:?}"
    );
}

#[test]
fn loader_errors_on_out_of_range_face_normal() {
    let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1//2 2//2 3//2\n";
    let err = load_obj(obj).unwrap_err();
    assert!(
        format!("{err:?}").contains("normal index"),
        "expected normal index error, got {err:?}"
    );
}

#[test]
fn loader_errors_on_face_with_less_than_three_corners() {
    let obj = "v 0 0 0\nv 1 0 0\nf 1 2\n";
    let err = load_obj(obj).unwrap_err();
    assert!(
        format!("{err:?}").contains("at least 3"),
        "expected too-few-corners error, got {err:?}"
    );
}

// ── #21  Saver must not panic on an out-of-range material index ───────────────

#[test]
fn saver_handles_out_of_range_material_index_without_panicking() {
    let mut b = SceneBuilder::new();
    let mut mesh = Mesh::new("BadMat");
    mesh.vertices = (0..3)
        .map(|i| Vertex::new(glam::Vec3::new(i as f32, 0.0, 0.0)))
        .collect();
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(5))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene = b.build();

    let mut buf = Vec::new();
    ObjSaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(
        text.contains("usemtl (none)"),
        "out-of-range material must degrade to (none):\n{text}"
    );
}
