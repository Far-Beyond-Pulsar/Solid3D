//! Shared scene factories and round-trip helpers for solid-ply integration tests.

#![allow(dead_code)]

use glam::{Vec2, Vec3, Vec4};
use solid_ply::{PlyLoader, PlySaver};
use solid_rs::prelude::*;
use std::io::Cursor;

// ── Scene factories ───────────────────────────────────────────────────────────

/// 3 vertices (no normals/colors/uvs), 1 triangle primitive.
pub fn triangle_scene() -> Scene {
    let mut b = SceneBuilder::named("Triangle");
    let mut mesh = Mesh::new("Tri");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// 5 vertices, no primitives (point cloud).
pub fn point_cloud_scene() -> Scene {
    let mut b = SceneBuilder::named("PointCloud");
    let mut mesh = Mesh::new("Cloud");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, 0.0, 0.0)),
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(0.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(1.0, 1.0, 1.0)),
    ];
    mesh.primitives = vec![];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// 3 vertices each with colors[0] set, 1 triangle.
pub fn colored_vertex_scene() -> Scene {
    let mut b = SceneBuilder::named("Colored");
    let mut mesh = Mesh::new("ColorMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_color(Vec4::new(1.0, 0.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_color(Vec4::new(0.0, 1.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_color(Vec4::new(0.0, 0.0, 1.0, 1.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// 3 vertices each with uvs[0] set, 1 triangle.
pub fn uv_scene() -> Scene {
    let mut b = SceneBuilder::named("UV");
    let mut mesh = Mesh::new("UVMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_uv(Vec2::new(0.5, 1.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_uv(Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_uv(Vec2::new(1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// 3 vertices each with uvs[0] AND uvs[1] set, 1 triangle.
pub fn multi_uv_scene() -> Scene {
    let mut b = SceneBuilder::named("MultiUV");
    let mut mesh = Mesh::new("MultiUVMesh");
    let mut v0 = Vertex::new(Vec3::new(0.0, 1.0, 0.0));
    v0.uvs[0] = Some(Vec2::new(0.5, 1.0));
    v0.uvs[1] = Some(Vec2::new(0.25, 0.75));
    let mut v1 = Vertex::new(Vec3::new(-1.0, -1.0, 0.0));
    v1.uvs[0] = Some(Vec2::new(0.0, 0.0));
    v1.uvs[1] = Some(Vec2::new(0.1, 0.2));
    let mut v2 = Vertex::new(Vec3::new(1.0, -1.0, 0.0));
    v2.uvs[0] = Some(Vec2::new(1.0, 0.0));
    v2.uvs[1] = Some(Vec2::new(0.9, 0.8));
    mesh.vertices = vec![v0, v1, v2];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// 3 vertices each with tangent: Some(Vec4), 1 triangle.
pub fn tangent_scene() -> Scene {
    let mut b = SceneBuilder::named("Tangent");
    let mut mesh = Mesh::new("TangentMesh");
    let mut v0 = Vertex::new(Vec3::new(0.0, 1.0, 0.0));
    v0.tangent = Some(Vec4::new(1.0, 0.0, 0.0, 1.0));
    let mut v1 = Vertex::new(Vec3::new(-1.0, -1.0, 0.0));
    v1.tangent = Some(Vec4::new(1.0, 0.0, 0.0, 1.0));
    let mut v2 = Vertex::new(Vec3::new(1.0, -1.0, 0.0));
    v2.tangent = Some(Vec4::new(1.0, 0.0, 0.0, 1.0));
    mesh.vertices = vec![v0, v1, v2];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// 3 vertices each with normal: Some(Vec3::Z), 1 triangle.
pub fn normal_scene() -> Scene {
    let mut b = SceneBuilder::named("Normal");
    let mut mesh = Mesh::new("NormalMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

// ── Round-trip helpers ────────────────────────────────────────────────────────

pub fn ascii_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::new();
    PlySaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap()
}

pub fn binary_le_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::new();
    PlySaver::save_binary_le(scene, &mut buf, &SaveOptions::default()).unwrap();
    PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap()
}

pub fn binary_be_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::new();
    PlySaver::save_binary_be(scene, &mut buf, &SaveOptions::default()).unwrap();
    PlyLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap()
}

// ── Binary test utilities ─────────────────────────────────────────────────────

/// Returns the byte slice that follows "end_header\n".
pub fn data_after_header(buf: &[u8]) -> &[u8] {
    let marker = b"end_header\n";
    let pos = buf
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("end_header not found");
    &buf[pos + marker.len()..]
}

/// Extracts the header text (everything before "end_header").
pub fn header_text(buf: &[u8]) -> String {
    let marker = b"end_header";
    let pos = buf
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("end_header not found");
    String::from_utf8(buf[..pos].to_vec()).unwrap()
}
