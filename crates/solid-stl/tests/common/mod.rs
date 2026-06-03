//! Shared helpers for solid-stl integration tests.

#![allow(dead_code)]

use glam::{Vec3, Vec4};
use solid_rs::prelude::*;
use solid_stl::{StlLoader, StlSaver};
use std::io::Cursor;

// ── Low-level binary write helpers ───────────────────────────────────────────

pub fn write_f32_le(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn write_u16_le(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn read_f32_le(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn read_vec3(data: &[u8], offset: usize) -> Vec3 {
    Vec3::new(
        read_f32_le(data, offset),
        read_f32_le(data, offset + 4),
        read_f32_le(data, offset + 8),
    )
}

pub fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

// ── Binary STL fixtures ───────────────────────────────────────────────────────

/// One triangle in the XY plane: (0,0,0), (1,0,0), (0,1,0).  Normal = +Z.
pub fn single_triangle_binary() -> Vec<u8> {
    let mut buf = vec![0u8; 80]; // header
    write_u32_le(&mut buf, 1); // 1 triangle
                               // face normal
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    // v0
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    // v1
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    // v2
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_u16_le(&mut buf, 0); // attribute byte count
    buf
}

/// ASCII STL with one triangle: (0,0,0), (1,0,0), (0,1,0).
pub fn single_triangle_ascii() -> Vec<u8> {
    b"solid test\n\
      facet normal 0.000000 0.000000 1.000000\n\
        outer loop\n\
          vertex 0.000000 0.000000 0.000000\n\
          vertex 1.000000 0.000000 0.000000\n\
          vertex 0.000000 1.000000 0.000000\n\
        endloop\n\
      endfacet\n\
    endsolid test\n"
        .to_vec()
}

/// Two triangles forming a unit quad in the XY plane.
/// Triangle 0: (0,0,0)-(1,0,0)-(0,1,0)
/// Triangle 1: (1,0,0)-(1,1,0)-(0,1,0)
/// Unique positions: 4  →  indices: 6 after dedup.
pub fn two_triangle_binary() -> Vec<u8> {
    let mut buf = vec![0u8; 80];
    write_u32_le(&mut buf, 2);
    // Triangle 0
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_u16_le(&mut buf, 0);
    // Triangle 1
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_u16_le(&mut buf, 0);
    buf
}

/// One triangle with a VisCAM RGB555 color in the attribute bytes.
/// `r`, `g`, `b` are 5-bit values (0–31).
pub fn colored_triangle_binary(r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut buf = vec![0u8; 80];
    write_u32_le(&mut buf, 1);
    // normal
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    // vertices
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 0.0);
    write_f32_le(&mut buf, 1.0);
    write_f32_le(&mut buf, 0.0);
    // VisCAM: bit 15 = color-valid, bits 14-10 = R, 9-5 = G, 4-0 = B
    let r5 = (r & 0x1F) as u16;
    let g5 = (g & 0x1F) as u16;
    let b5 = (b & 0x1F) as u16;
    let attr: u16 = 0x8000 | (r5 << 10) | (g5 << 5) | b5;
    write_u16_le(&mut buf, attr);
    buf
}

// ── Round-trip helpers ────────────────────────────────────────────────────────

/// Save `scene` as binary STL, then load and return the result.
pub fn binary_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::<u8>::new();
    StlSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    StlLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap()
}

/// Save `scene` as ASCII STL, then load and return the result.
pub fn ascii_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::<u8>::new();
    StlSaver
        .save_ascii(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    StlLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap()
}

// ── Scene factories ───────────────────────────────────────────────────────────

/// Build a single-triangle scene from three explicit positions.
pub fn triangle_scene(p0: Vec3, p1: Vec3, p2: Vec3) -> Scene {
    let mut b = SceneBuilder::named("Triangle Scene");
    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![Vertex::new(p0), Vertex::new(p1), Vertex::new(p2)];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let root = b.add_root_node("Root");
    b.attach_mesh(root, mi);
    b.build()
}

/// Build a single-triangle scene whose first vertex carries a VisCAM color.
pub fn colored_triangle_scene(color: Vec4) -> Scene {
    let mut b = SceneBuilder::named("Colored Scene");
    let mut mesh = Mesh::new("ColoredTri");
    let mut v0 = Vertex::new(Vec3::new(0.0, 0.0, 0.0));
    v0.colors[0] = Some(color);
    let mut v1 = Vertex::new(Vec3::new(1.0, 0.0, 0.0));
    v1.colors[0] = Some(color);
    let mut v2 = Vertex::new(Vec3::new(0.0, 1.0, 0.0));
    v2.colors[0] = Some(color);
    mesh.vertices = vec![v0, v1, v2];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let root = b.add_root_node("Root");
    b.attach_mesh(root, mi);
    b.build()
}

/// Count total triangles across all meshes / primitives of a scene.
pub fn total_triangle_count(scene: &Scene) -> usize {
    scene
        .meshes
        .iter()
        .flat_map(|m| m.primitives.iter())
        .map(|p| p.indices.len() / 3)
        .sum()
}
