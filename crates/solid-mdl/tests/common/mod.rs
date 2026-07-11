//! Shared helpers for solid-mdl integration tests.

#![allow(dead_code)]

use glam::Vec3;
use solid_rs::prelude::*;
use solid_mdl::{MdlLoader, MdlSaver};
use std::io::Cursor;

// ── Low-level binary write helpers ─────────────────────────────────────────────

pub fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn write_i32_le(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn write_f32_le(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

pub fn write_f32_3(buf: &mut Vec<u8>, v: [f32; 3]) {
    write_f32_le(buf, v[0]);
    write_f32_le(buf, v[1]);
    write_f32_le(buf, v[2]);
}

pub fn write_i32_3(buf: &mut Vec<u8>, v: [i32; 3]) {
    write_i32_le(buf, v[0]);
    write_i32_le(buf, v[1]);
    write_i32_le(buf, v[2]);
}

// ── MDL fixture: single triangle ──────────────────────────────────────────────
//
// A minimal valid MDL with one triangle in the first frame, no skins.
// Vertex 0: (0,0,0)  Vertex 1: (1,0,0)  Vertex 2: (0,1,0)
// scale = (1/255, 1/255, 1/255) → decompress: v_real = v_byte / 255

pub fn single_triangle_mdl() -> Vec<u8> {
    let mut buf = Vec::new();

    // Header
    write_u32_le(&mut buf, 1330660425); // ident "IDPO"
    write_i32_le(&mut buf, 6);          // version
    write_f32_3(&mut buf, [1.0 / 255.0, 1.0 / 255.0, 1.0 / 255.0]); // scale
    write_f32_3(&mut buf, [0.0, 0.0, 0.0]); // translate
    write_f32_le(&mut buf, 1.0);        // bounding radius
    write_f32_3(&mut buf, [0.0, 0.0, 0.0]); // eyeposition
    write_i32_le(&mut buf, 0);          // num_skins
    write_i32_le(&mut buf, 64);         // skinwidth
    write_i32_le(&mut buf, 64);         // skinheight
    write_i32_le(&mut buf, 3);          // num_verts
    write_i32_le(&mut buf, 1);          // num_tris
    write_i32_le(&mut buf, 1);          // num_frames
    write_i32_le(&mut buf, 0);          // synctype
    write_i32_le(&mut buf, 0);          // flags
    write_f32_le(&mut buf, 0.0);        // size

    // No skins

    // Texture coords (3 vertices)
    for _ in 0..3 {
        write_i32_le(&mut buf, 0);  // onseam
        write_i32_le(&mut buf, 0);  // s
        write_i32_le(&mut buf, 0);  // t
    }

    // Triangles (1)
    write_i32_le(&mut buf, 1);  // facesfront
    write_i32_le(&mut buf, 0);  // vertex[0]
    write_i32_le(&mut buf, 1);  // vertex[1]
    write_i32_le(&mut buf, 2);  // vertex[2]

    // Frame: simple frame
    write_i32_le(&mut buf, 0);  // type = simple
    // bboxmin
    buf.extend_from_slice(&[0, 0, 0, 0]);
    // bboxmax
    buf.extend_from_slice(&[255, 255, 255, 0]);
    // name
    buf.extend_from_slice(b"frame_00        ");
    // Vertices: v0=(0,0,0), v1=(255,0,0), v2=(0,255,0) → decompressed positions
    // v0 = (0, 0, 0)
    buf.extend_from_slice(&[0, 0, 0, 0]);
    // v1 = (1, 0, 0)
    buf.extend_from_slice(&[255, 0, 0, 0]);
    // v2 = (0, 1, 0)
    buf.extend_from_slice(&[0, 255, 0, 0]);

    buf
}

// ── Round-trip helpers ────────────────────────────────────────────────────────

pub fn round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::new();
    MdlSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    MdlLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .unwrap()
}

pub fn triangle_scene(p0: Vec3, p1: Vec3, p2: Vec3) -> Scene {
    let mut b = SceneBuilder::named("MDL Scene");
    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![Vertex::new(p0), Vertex::new(p1), Vertex::new(p2)];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let root = b.add_root_node("Root");
    b.attach_mesh(root, mi);
    b.build()
}

pub fn total_triangle_count(scene: &Scene) -> usize {
    scene
        .meshes
        .iter()
        .flat_map(|m| m.primitives.iter())
        .map(|p| p.indices.len() / 3)
        .sum()
}
