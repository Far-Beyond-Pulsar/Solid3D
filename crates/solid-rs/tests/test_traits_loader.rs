mod common;
use common::*;
use solid_rs::prelude::*;
use std::io::Cursor;

// ══════════════════════════════════════════════════════════════════════════════
// LoadOptions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn load_options_default_generate_normals_false() {
    assert!(!LoadOptions::default().generate_normals);
}

#[test]
fn load_options_default_triangulate_false() {
    assert!(!LoadOptions::default().triangulate);
}

#[test]
fn load_options_default_merge_vertices_false() {
    assert!(!LoadOptions::default().merge_vertices);
}

#[test]
fn load_options_default_flip_uv_v_false() {
    assert!(!LoadOptions::default().flip_uv_v);
}

#[test]
fn load_options_default_max_texture_size_none() {
    assert!(LoadOptions::default().max_texture_size.is_none());
}

#[test]
fn load_options_default_base_dir_none() {
    assert!(LoadOptions::default().base_dir.is_none());
}

#[test]
fn load_options_can_be_customised() {
    let opts = LoadOptions {
        generate_normals: true,
        triangulate: true,
        merge_vertices: true,
        flip_uv_v: true,
        max_texture_size: Some(1024),
        base_dir: Some("/tmp".into()),
    };
    assert!(opts.generate_normals);
    assert!(opts.triangulate);
    assert!(opts.merge_vertices);
    assert!(opts.flip_uv_v);
    assert_eq!(opts.max_texture_size, Some(1024));
    assert_eq!(
        opts.base_dir.as_ref().and_then(|p| p.to_str()),
        Some("/tmp")
    );
}

#[test]
fn load_options_clone() {
    let o1 = LoadOptions {
        generate_normals: true,
        ..Default::default()
    };
    let o2 = o1.clone();
    assert_eq!(o1.generate_normals, o2.generate_normals);
}

// ══════════════════════════════════════════════════════════════════════════════
// Loader trait - MockLoader
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn loader_format_info_correct() {
    let info = MockLoader.format_info();
    assert_eq!(info.id, "mock");
    assert!(info.can_load);
}

#[test]
fn loader_load_returns_scene() {
    let mut data = Cursor::new(b"");
    let scene = MockLoader.load(&mut data, &LoadOptions::default()).unwrap();
    assert!(!scene.nodes.is_empty());
}

#[test]
fn loader_format_info_name() {
    assert_eq!(MockLoader.format_info().name, "Mock Format");
}

#[test]
fn loader_default_detect_returns_low() {
    // MockLoader.detect() reads magic "MOCK" header
    let mut data = Cursor::new(b"XXXX");
    assert_eq!(MockLoader.detect(&mut data), 0.0);
}

#[test]
fn loader_detect_magic_bytes() {
    let mut data = Cursor::new(b"MOCK rest of file");
    assert!(MockLoader.detect(&mut data) > 0.5);
}

// ══════════════════════════════════════════════════════════════════════════════
// Loader trait - FailLoader
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn fail_loader_returns_error() {
    let mut data = Cursor::new(b"");
    let r = FailLoader.load(&mut data, &LoadOptions::default());
    assert!(r.is_err());
}

#[test]
fn fail_loader_error_is_parse() {
    let mut data = Cursor::new(b"");
    let e = FailLoader
        .load(&mut data, &LoadOptions::default())
        .unwrap_err();
    assert!(matches!(e, SolidError::Parse(_)));
}

// ══════════════════════════════════════════════════════════════════════════════
// XyzLoader
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn xyz_loader_empty_input_empty_mesh() {
    let scene = XyzLoader
        .load(&mut Cursor::new(b""), &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].vertex_count(), 0);
}

#[test]
fn xyz_loader_one_vertex() {
    let data = b"1.0 2.0 3.0\n";
    let scene = XyzLoader
        .load(&mut Cursor::new(data), &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].vertex_count(), 1);
}

#[test]
fn xyz_loader_three_vertices() {
    let data = b"0 0 0\n1 0 0\n0 1 0\n";
    let scene = XyzLoader
        .load(&mut Cursor::new(data), &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].vertex_count(), 3);
}

#[test]
fn xyz_loader_positions_correct() {
    use glam::Vec3;
    let data = b"4.0 5.0 6.0\n";
    let scene = XyzLoader
        .load(&mut Cursor::new(data), &LoadOptions::default())
        .unwrap();
    assert_eq!(
        scene.meshes[0].vertices[0].position,
        Vec3::new(4.0, 5.0, 6.0)
    );
}

#[test]
fn xyz_loader_malformed_returns_error() {
    let data = b"not a number here\n";
    let r = XyzLoader.load(&mut Cursor::new(data), &LoadOptions::default());
    assert!(r.is_err());
}
