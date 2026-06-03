mod common;
use common::*;

use glam::{Vec2, Vec3, Vec4};
use solid_ply::PlySaver;
use solid_rs::prelude::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn save_ascii(scene: &Scene) -> String {
    let mut buf = Vec::new();
    PlySaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    String::from_utf8(buf).unwrap()
}

fn save_binary_le(scene: &Scene) -> Vec<u8> {
    let mut buf = Vec::new();
    PlySaver::save_binary_le(scene, &mut buf, &SaveOptions::default()).unwrap();
    buf
}

fn save_binary_be(scene: &Scene) -> Vec<u8> {
    let mut buf = Vec::new();
    PlySaver::save_binary_be(scene, &mut buf, &SaveOptions::default()).unwrap();
    buf
}

/// Extract vertex data lines (after "end_header\n").
fn vertex_lines(text: &str) -> Vec<Vec<f32>> {
    let after = text.split("end_header\n").nth(1).unwrap_or("");
    after
        .lines()
        .filter(|l| !l.starts_with("3 ") && !l.is_empty())
        .map(|l| {
            l.split_whitespace()
                .map(|t| t.parse::<f32>().unwrap())
                .collect()
        })
        .collect()
}

// ── ASCII tests ───────────────────────────────────────────────────────────────

#[test]
fn saver_ascii_starts_with_ply() {
    let text = save_ascii(&triangle_scene());
    assert!(text.starts_with("ply\n"), "output must start with 'ply\\n'");
}

#[test]
fn saver_ascii_format_line() {
    let text = save_ascii(&triangle_scene());
    assert!(
        text.contains("format ascii 1.0"),
        "ASCII output must contain 'format ascii 1.0'"
    );
}

#[test]
fn saver_ascii_element_vertex_count() {
    let text = save_ascii(&triangle_scene());
    assert!(
        text.contains("element vertex 3"),
        "triangle scene must declare 3 vertices"
    );
}

#[test]
fn saver_ascii_element_face_count() {
    let text = save_ascii(&triangle_scene());
    assert!(
        text.contains("element face 1"),
        "single-triangle scene must declare 1 face"
    );
}

#[test]
fn saver_ascii_positions_correct() {
    let text = save_ascii(&triangle_scene());
    let vlines = vertex_lines(&text);
    assert_eq!(vlines.len(), 3, "should have 3 vertex lines");
    let eps = 1e-5_f32;
    // v0 = (0, 1, 0)
    assert!((vlines[0][0] - 0.0).abs() < eps);
    assert!((vlines[0][1] - 1.0).abs() < eps);
    assert!((vlines[0][2] - 0.0).abs() < eps);
    // v1 = (-1, -1, 0)
    assert!((vlines[1][0] - (-1.0)).abs() < eps);
    assert!((vlines[1][1] - (-1.0)).abs() < eps);
    // v2 = (1, -1, 0)
    assert!((vlines[2][0] - 1.0).abs() < eps);
}

#[test]
fn saver_ascii_normals_emitted_when_present() {
    let text = save_ascii(&normal_scene());
    assert!(text.contains("property float nx"), "nx should be in header");
    assert!(text.contains("property float ny"), "ny should be in header");
    assert!(text.contains("property float nz"), "nz should be in header");
}

#[test]
fn saver_ascii_normals_omitted_when_absent() {
    let text = save_ascii(&triangle_scene());
    assert!(
        !text.contains("property float nx"),
        "nx must not appear when no vertex has a normal"
    );
}

#[test]
fn saver_ascii_colors_emitted_when_present() {
    let text = save_ascii(&colored_vertex_scene());
    assert!(text.contains("property uchar red"), "red property missing");
    assert!(
        text.contains("property uchar green"),
        "green property missing"
    );
    assert!(
        text.contains("property uchar blue"),
        "blue property missing"
    );
    assert!(
        text.contains("property uchar alpha"),
        "alpha property missing"
    );
}

#[test]
fn saver_ascii_uvs_emitted_channel_0() {
    let text = save_ascii(&uv_scene());
    assert!(text.contains("property float s\n"), "s property missing");
    assert!(text.contains("property float t\n"), "t property missing");
}

#[test]
fn saver_ascii_uvs_emitted_channel_1() {
    let text = save_ascii(&multi_uv_scene());
    assert!(text.contains("property float s1\n"), "s1 property missing");
    assert!(text.contains("property float t1\n"), "t1 property missing");
}

#[test]
fn saver_ascii_tangents_emitted_when_present() {
    let text = save_ascii(&tangent_scene());
    assert!(text.contains("property float tx"), "tx property missing");
    assert!(text.contains("property float ty"), "ty property missing");
    assert!(text.contains("property float tz"), "tz property missing");
    assert!(text.contains("property float tw"), "tw property missing");
}

#[test]
fn saver_ascii_point_cloud_no_face_element() {
    let text = save_ascii(&point_cloud_scene());
    assert!(
        !text.contains("element face"),
        "point cloud must not emit a face element"
    );
}

// ── Binary LE tests ───────────────────────────────────────────────────────────

#[test]
fn saver_binary_le_header_format_line() {
    let buf = save_binary_le(&triangle_scene());
    let hdr = header_text(&buf);
    assert!(
        hdr.contains("format binary_little_endian 1.0"),
        "LE header must contain 'format binary_little_endian 1.0'"
    );
}

#[test]
fn saver_binary_le_positions_correct() {
    let buf = save_binary_le(&triangle_scene());
    let data = data_after_header(&buf);
    // No normals/colors/uvs → 3 floats (x,y,z) per vertex, LE
    let x0 = f32::from_le_bytes(data[0..4].try_into().unwrap());
    let y0 = f32::from_le_bytes(data[4..8].try_into().unwrap());
    let z0 = f32::from_le_bytes(data[8..12].try_into().unwrap());
    let eps = 1e-6_f32;
    assert!((x0 - 0.0).abs() < eps, "v0.x = {x0}");
    assert!((y0 - 1.0).abs() < eps, "v0.y = {y0}");
    assert!((z0 - 0.0).abs() < eps, "v0.z = {z0}");
    // v1
    let x1 = f32::from_le_bytes(data[12..16].try_into().unwrap());
    assert!((x1 - (-1.0)).abs() < eps, "v1.x = {x1}");
}

// ── Binary BE tests ───────────────────────────────────────────────────────────

#[test]
fn saver_binary_be_header_format_line() {
    let buf = save_binary_be(&triangle_scene());
    let hdr = header_text(&buf);
    assert!(
        hdr.contains("format binary_big_endian 1.0"),
        "BE header must contain 'format binary_big_endian 1.0'"
    );
}

#[test]
fn saver_binary_be_positions_correct() {
    let buf = save_binary_be(&triangle_scene());
    let data = data_after_header(&buf);
    let x0 = f32::from_be_bytes(data[0..4].try_into().unwrap());
    let y0 = f32::from_be_bytes(data[4..8].try_into().unwrap());
    let z0 = f32::from_be_bytes(data[8..12].try_into().unwrap());
    let eps = 1e-6_f32;
    assert!((x0 - 0.0).abs() < eps, "v0.x = {x0}");
    assert!((y0 - 1.0).abs() < eps, "v0.y = {y0}");
    assert!((z0 - 0.0).abs() < eps, "v0.z = {z0}");
}

// ── Double precision tests ────────────────────────────────────────────────────

#[test]
fn saver_double_precision_header_uses_double() {
    let mut buf = Vec::new();
    PlySaver::save_with_precision(&triangle_scene(), &mut buf, true).unwrap();
    let hdr = header_text(&buf);
    assert!(hdr.contains("property double x"), "x should use 'double'");
    assert!(hdr.contains("property double y"), "y should use 'double'");
    assert!(hdr.contains("property double z"), "z should use 'double'");
}

#[test]
fn saver_double_precision_values_correct() {
    let mut buf = Vec::new();
    PlySaver::save_with_precision(&triangle_scene(), &mut buf, true).unwrap();
    let data = data_after_header(&buf);
    // LE doubles: 8 bytes per value
    let x0 = f64::from_le_bytes(data[0..8].try_into().unwrap());
    let y0 = f64::from_le_bytes(data[8..16].try_into().unwrap());
    let z0 = f64::from_le_bytes(data[16..24].try_into().unwrap());
    let eps = 1e-10_f64;
    assert!((x0 - 0.0).abs() < eps, "v0.x = {x0}");
    assert!((y0 - 1.0).abs() < eps, "v0.y = {y0}");
    assert!((z0 - 0.0).abs() < eps, "v0.z = {z0}");
}

// ── Multi-mesh and edge-case tests ────────────────────────────────────────────

#[test]
fn saver_multiple_meshes_total_vertex_count() {
    let mut b = SceneBuilder::named("Two Meshes");
    let mut m1 = Mesh::new("Mesh1");
    m1.vertices = vec![
        Vertex::new(Vec3::ZERO),
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
    ];
    m1.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mut m2 = Mesh::new("Mesh2");
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

    let text = save_ascii(&scene);
    assert!(
        text.contains("element vertex 6"),
        "two 3-vert meshes should merge to 6 vertices"
    );
    assert!(
        text.contains("element face 2"),
        "two triangles should produce 2 faces"
    );
}

#[test]
fn saver_empty_scene_valid_ply() {
    let scene = Scene::new();
    let mut buf = Vec::new();
    PlySaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(
        text.starts_with("ply\n"),
        "empty scene must still produce valid PLY"
    );
    assert!(
        text.contains("element vertex 0"),
        "should declare 0 vertices"
    );
    assert!(
        !text.contains("element face"),
        "no face element for empty scene"
    );
}
