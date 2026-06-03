mod common;

use solid_ply::PlyLoader;
use solid_rs::prelude::*;
use std::io::Cursor;

// ── Inline PLY fixtures ───────────────────────────────────────────────────────

const SIMPLE_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 3\n\
property float x\n\
property float y\n\
property float z\n\
element face 1\n\
property list uchar uint vertex_indices\n\
end_header\n\
0 0 0\n\
1 0 0\n\
0 1 0\n\
3 0 1 2\n";

const NORMAL_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 3\n\
property float x\n\
property float y\n\
property float z\n\
property float nx\n\
property float ny\n\
property float nz\n\
element face 1\n\
property list uchar uint vertex_indices\n\
end_header\n\
0 0 0 0 0 1\n\
1 0 0 0 0 1\n\
0 1 0 0 0 1\n\
3 0 1 2\n";

const COLOR_UCHAR_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 3\n\
property float x\n\
property float y\n\
property float z\n\
property uchar red\n\
property uchar green\n\
property uchar blue\n\
property uchar alpha\n\
element face 1\n\
property list uchar uint vertex_indices\n\
end_header\n\
0 0 0 255 0 0 255\n\
1 0 0 0 255 0 255\n\
0 1 0 0 0 255 255\n\
3 0 1 2\n";

const UV_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 3\n\
property float x\n\
property float y\n\
property float z\n\
property float s\n\
property float t\n\
element face 1\n\
property list uchar uint vertex_indices\n\
end_header\n\
0 0 0 0.5 0.25\n\
1 0 0 1 0\n\
0 1 0 0 1\n\
3 0 1 2\n";

const POINT_CLOUD_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 4\n\
property float x\n\
property float y\n\
property float z\n\
end_header\n\
0 0 0\n\
1 0 0\n\
0 1 0\n\
0 0 1\n";

const DOUBLE_PRECISION_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 2\n\
property double x\n\
property double y\n\
property double z\n\
element face 1\n\
property list uchar uint vertex_indices\n\
end_header\n\
1.5 2.5 3.5\n\
4.5 5.5 6.5\n\
2 0 1\n";

const UV_CHANNEL1_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 2\n\
property float x\n\
property float y\n\
property float z\n\
property float s1\n\
property float t1\n\
element face 0\n\
property list uchar uint vertex_indices\n\
end_header\n\
0 0 0 0.3 0.7\n\
1 1 1 0.6 0.4\n";

const TANGENT_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 3\n\
property float x\n\
property float y\n\
property float z\n\
property float tx\n\
property float ty\n\
property float tz\n\
property float tw\n\
element face 1\n\
property list uchar uint vertex_indices\n\
end_header\n\
2 3 4 1 0 0 1\n\
5 6 7 1 0 0 1\n\
8 9 10 1 0 0 1\n\
3 0 1 2\n";

const ALPHA_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 1\n\
property float x\n\
property float y\n\
property float z\n\
property uchar red\n\
property uchar green\n\
property uchar blue\n\
property uchar alpha\n\
element face 0\n\
property list uchar uint vertex_indices\n\
end_header\n\
0 0 0 0 0 0 128\n";

const MULTI_MESH_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 6\n\
property float x\n\
property float y\n\
property float z\n\
element face 2\n\
property list uchar uint vertex_indices\n\
end_header\n\
0 0 0\n\
1 0 0\n\
0 1 0\n\
2 0 0\n\
3 0 0\n\
2 1 0\n\
3 0 1 2\n\
3 3 4 5\n";

const EMPTY_ELEMENTS_PLY: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 0\n\
property float x\n\
property float y\n\
property float z\n\
element face 0\n\
property list uchar uint vertex_indices\n\
end_header\n";

fn load_str(ply: &str) -> Result<Scene> {
    PlyLoader.load(&mut Cursor::new(ply.as_bytes()), &LoadOptions::default())
}

fn simple_binary_le_ply() -> Vec<u8> {
    let header = "ply\nformat binary_little_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n";
    let mut buf: Vec<u8> = header.as_bytes().to_vec();
    buf.extend_from_slice(&0.0f32.to_le_bytes());
    buf.extend_from_slice(&0.0f32.to_le_bytes());
    buf.extend_from_slice(&0.0f32.to_le_bytes());
    buf.extend_from_slice(&1.0f32.to_le_bytes());
    buf.extend_from_slice(&0.0f32.to_le_bytes());
    buf.extend_from_slice(&0.0f32.to_le_bytes());
    buf.extend_from_slice(&0.0f32.to_le_bytes());
    buf.extend_from_slice(&1.0f32.to_le_bytes());
    buf.extend_from_slice(&0.0f32.to_le_bytes());
    buf.push(3u8);
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf
}

fn simple_binary_be_ply() -> Vec<u8> {
    let header = "ply\nformat binary_big_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n";
    let mut buf: Vec<u8> = header.as_bytes().to_vec();
    buf.extend_from_slice(&0.0f32.to_be_bytes());
    buf.extend_from_slice(&0.0f32.to_be_bytes());
    buf.extend_from_slice(&0.0f32.to_be_bytes());
    buf.extend_from_slice(&1.0f32.to_be_bytes());
    buf.extend_from_slice(&0.0f32.to_be_bytes());
    buf.extend_from_slice(&0.0f32.to_be_bytes());
    buf.extend_from_slice(&0.0f32.to_be_bytes());
    buf.extend_from_slice(&1.0f32.to_be_bytes());
    buf.extend_from_slice(&0.0f32.to_be_bytes());
    buf.push(3u8);
    buf.extend_from_slice(&0u32.to_be_bytes());
    buf.extend_from_slice(&1u32.to_be_bytes());
    buf.extend_from_slice(&2u32.to_be_bytes());
    buf
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn loader_ascii_ply_header_parsed() {
    let scene = load_str(SIMPLE_PLY).unwrap();
    assert_eq!(scene.meshes.len(), 1);
}

#[test]
fn loader_ascii_single_triangle_vertices() {
    let scene = load_str(SIMPLE_PLY).unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_ascii_single_triangle_faces() {
    let scene = load_str(SIMPLE_PLY).unwrap();
    let prim = &scene.meshes[0].primitives[0];
    // 1 triangle = 3 indices
    assert_eq!(prim.indices.len(), 3);
    assert_eq!(prim.indices[0], 0);
    assert_eq!(prim.indices[1], 1);
    assert_eq!(prim.indices[2], 2);
}

#[test]
fn loader_ascii_positions_correct() {
    let scene = load_str(SIMPLE_PLY).unwrap();
    let verts = &scene.meshes[0].vertices;
    let eps = 1e-6_f32;
    assert!((verts[0].position.x - 0.0).abs() < eps);
    assert!((verts[0].position.y - 0.0).abs() < eps);
    assert!((verts[0].position.z - 0.0).abs() < eps);
    assert!((verts[1].position.x - 1.0).abs() < eps);
    assert!((verts[2].position.y - 1.0).abs() < eps);
}

#[test]
fn loader_ascii_normals_loaded() {
    let scene = load_str(NORMAL_PLY).unwrap();
    let v0 = &scene.meshes[0].vertices[0];
    let n = v0.normal.expect("normal should be present");
    let eps = 1e-6_f32;
    assert!((n.x - 0.0).abs() < eps);
    assert!((n.y - 0.0).abs() < eps);
    assert!((n.z - 1.0).abs() < eps);
}

#[test]
fn loader_ascii_colors_loaded_uchar() {
    let scene = load_str(COLOR_UCHAR_PLY).unwrap();
    let c = scene.meshes[0].vertices[0].colors[0].expect("color should be present");
    let eps = 1.0_f32 / 255.0;
    assert!(
        (c.x - 1.0).abs() < eps,
        "red channel should be ~1.0, got {}",
        c.x
    );
    assert!(
        (c.y - 0.0).abs() < eps,
        "green channel should be ~0.0, got {}",
        c.y
    );
    assert!(
        (c.z - 0.0).abs() < eps,
        "blue channel should be ~0.0, got {}",
        c.z
    );
    assert!(
        (c.w - 1.0).abs() < eps,
        "alpha channel should be ~1.0, got {}",
        c.w
    );
}

#[test]
fn loader_ascii_uvs_loaded() {
    let scene = load_str(UV_PLY).unwrap();
    let uv = scene.meshes[0].vertices[0].uvs[0].expect("uv[0] should be present");
    let eps = 1e-6_f32;
    assert!((uv.x - 0.5).abs() < eps, "s should be 0.5, got {}", uv.x);
    assert!((uv.y - 0.25).abs() < eps, "t should be 0.25, got {}", uv.y);
}

#[test]
fn loader_ascii_point_cloud_no_faces() {
    let scene = load_str(POINT_CLOUD_PLY).unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 4);
    let has_triangles = scene.meshes[0]
        .primitives
        .iter()
        .any(|p| p.topology == Topology::TriangleList);
    assert!(!has_triangles, "point cloud should have no triangle faces");
}

#[test]
fn loader_binary_le_triangle_vertices() {
    let buf = simple_binary_le_ply();
    let scene = PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_binary_le_positions_correct() {
    let buf = simple_binary_le_ply();
    let scene = PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap();
    let verts = &scene.meshes[0].vertices;
    let eps = 1e-6_f32;
    // v0 = (0,0,0), v1 = (1,0,0), v2 = (0,1,0)
    assert!((verts[0].position.x - 0.0).abs() < eps);
    assert!((verts[1].position.x - 1.0).abs() < eps);
    assert!((verts[2].position.y - 1.0).abs() < eps);
}

#[test]
fn loader_binary_be_triangle_vertices() {
    let buf = simple_binary_be_ply();
    let scene = PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_binary_be_positions_correct() {
    let buf = simple_binary_be_ply();
    let scene = PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap();
    let verts = &scene.meshes[0].vertices;
    let eps = 1e-6_f32;
    assert!((verts[0].position.x - 0.0).abs() < eps);
    assert!((verts[1].position.x - 1.0).abs() < eps);
    assert!((verts[2].position.y - 1.0).abs() < eps);
}

#[test]
fn loader_double_precision_positions() {
    let scene = load_str(DOUBLE_PRECISION_PLY).unwrap();
    let verts = &scene.meshes[0].vertices;
    let eps = 1e-5_f32;
    assert!((verts[0].position.x - 1.5).abs() < eps);
    assert!((verts[0].position.y - 2.5).abs() < eps);
    assert!((verts[0].position.z - 3.5).abs() < eps);
    assert!((verts[1].position.x - 4.5).abs() < eps);
}

#[test]
fn loader_uv_channel_1_loaded() {
    let scene = load_str(UV_CHANNEL1_PLY).unwrap();
    let uv1 = scene.meshes[0].vertices[0].uvs[1].expect("uvs[1] should be present");
    let eps = 1e-6_f32;
    assert!((uv1.x - 0.3).abs() < eps, "s1 should be 0.3, got {}", uv1.x);
    assert!((uv1.y - 0.7).abs() < eps, "t1 should be 0.7, got {}", uv1.y);
}

#[test]
fn loader_tangents_loaded() {
    // Loader doesn't restore tangents, but must still correctly load positions
    // when tangent properties (tx ty tz tw) are present in the PLY.
    let scene = load_str(TANGENT_PLY).unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 3);
    let v0 = &scene.meshes[0].vertices[0];
    let eps = 1e-6_f32;
    assert!((v0.position.x - 2.0).abs() < eps);
    assert!((v0.position.y - 3.0).abs() < eps);
    assert!((v0.position.z - 4.0).abs() < eps);
}

#[test]
fn loader_alpha_channel_loaded() {
    let scene = load_str(ALPHA_PLY).unwrap();
    let c = scene.meshes[0].vertices[0].colors[0].expect("color should be present");
    let expected_alpha = 128.0_f32 / 255.0;
    let eps = 1.0_f32 / 255.0;
    assert!(
        (c.w - expected_alpha).abs() < eps,
        "alpha should be ~{expected_alpha}, got {}",
        c.w
    );
}

#[test]
fn loader_multiple_meshes_in_one_file() {
    // A single PLY with 6 verts and 2 triangles (simulating merged meshes).
    let scene = load_str(MULTI_MESH_PLY).unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 6);
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 6);
}

#[test]
fn loader_rejects_invalid_header() {
    let result = PlyLoader.load(
        &mut Cursor::new(b"this is not a ply file at all"),
        &LoadOptions::default(),
    );
    assert!(result.is_err(), "should reject invalid PLY data");
}

#[test]
fn loader_end_header_required() {
    let no_end_header = b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\n";
    let result = PlyLoader.load(&mut Cursor::new(no_end_header), &LoadOptions::default());
    assert!(result.is_err(), "should require end_header marker");
}

#[test]
fn loader_empty_element_lists() {
    let scene = load_str(EMPTY_ELEMENTS_PLY).unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 0);
}
