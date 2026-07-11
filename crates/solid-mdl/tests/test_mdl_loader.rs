//! Integration tests for MdlLoader.

mod common;

use common::*;
use solid_rs::prelude::*;
use solid_mdl::MdlLoader;
use std::io::Cursor;

#[test]
fn loader_detects_mdl_magic() {
    let data = single_triangle_mdl();
    let scene = MdlLoader
        .load(&mut Cursor::new(data), &LoadOptions::default())
        .expect("valid MDL should load");
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

#[test]
fn loader_rejects_bad_magic() {
    let mut data = single_triangle_mdl();
    data[0] = 0xFF; // corrupt magic
    let result = MdlLoader.load(&mut Cursor::new(data), &LoadOptions::default());
    assert!(result.is_err(), "bad magic should be rejected");
}

#[test]
fn loader_rejects_truncated() {
    let data = vec![0u8; 10];
    let result = MdlLoader.load(&mut Cursor::new(data), &LoadOptions::default());
    assert!(result.is_err(), "truncated MDL should be rejected");
}

#[test]
fn loader_single_triangle_vertex_count() {
    let scene = MdlLoader
        .load(
            &mut Cursor::new(single_triangle_mdl()),
            &LoadOptions::default(),
        )
        .unwrap();
    // 3 unique vertices
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_single_triangle_index_count() {
    let scene = MdlLoader
        .load(
            &mut Cursor::new(single_triangle_mdl()),
            &LoadOptions::default(),
        )
        .unwrap();
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

#[test]
fn loader_positions_correct() {
    let scene = MdlLoader
        .load(
            &mut Cursor::new(single_triangle_mdl()),
            &LoadOptions::default(),
        )
        .unwrap();
    let positions: Vec<_> = scene.meshes[0]
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    assert!(positions.contains(&glam::Vec3::new(0.0, 0.0, 0.0)));
    assert!(positions.contains(&glam::Vec3::new(1.0, 0.0, 0.0)));
    assert!(positions.contains(&glam::Vec3::new(0.0, 1.0, 0.0)));
}

#[test]
fn loader_normals_computed() {
    let scene = MdlLoader
        .load(
            &mut Cursor::new(single_triangle_mdl()),
            &LoadOptions::default(),
        )
        .unwrap();
    for v in &scene.meshes[0].vertices {
        assert!(v.normal.is_some(), "every vertex should have a normal");
    }
}

#[test]
fn loader_empty_scene_no_panic() {
    let mut buf = Vec::new();
    // Minimal header with zero counts
    write_u32_le(&mut buf, 1330660425); // ident "IDPO"
    write_i32_le(&mut buf, 6);          // version
    write_f32_3(&mut buf, [1.0; 3]);    // scale
    write_f32_3(&mut buf, [0.0; 3]);    // translate
    write_f32_le(&mut buf, 0.0);        // bounding radius
    write_f32_3(&mut buf, [0.0; 3]);    // eyeposition
    write_i32_le(&mut buf, 0);          // num_skins
    write_i32_le(&mut buf, 64);         // skinwidth
    write_i32_le(&mut buf, 64);         // skinheight
    write_i32_le(&mut buf, 0);          // num_verts = 0
    write_i32_le(&mut buf, 0);          // num_tris = 0
    write_i32_le(&mut buf, 0);          // num_frames = 0
    write_i32_le(&mut buf, 0);          // synctype
    write_i32_le(&mut buf, 0);          // flags
    write_f32_le(&mut buf, 0.0);        // size

    let scene = MdlLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .expect("empty MDL should not panic");
    assert!(scene.meshes.is_empty());
}
