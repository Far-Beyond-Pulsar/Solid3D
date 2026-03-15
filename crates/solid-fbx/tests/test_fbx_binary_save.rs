//! Integration tests — binary FBX save (`FbxSaver::save_binary`).

mod common;
use common::*;

use solid_fbx::{FbxLoader, FbxSaver};
use solid_rs::prelude::*;
use glam::*;
use std::io::Cursor;

const FBX_MAGIC: &[u8; 23] = b"Kaydara FBX Binary  \x00\x1a\x00";

// ── Magic / header ────────────────────────────────────────────────────────────

#[test]
fn binary_save_produces_fbx_magic_header() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    assert!(buf.len() >= 23, "output too short for magic header");
    assert_eq!(&buf[..23], FBX_MAGIC.as_slice(), "FBX magic header mismatch");
}

#[test]
fn binary_save_version_is_7400() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    assert!(buf.len() >= 27, "output too short for version field");
    let version = u32::from_le_bytes([buf[23], buf[24], buf[25], buf[26]]);
    assert_eq!(version, 7400, "expected FBX version 7400, got {}", version);
}

#[test]
fn binary_save_produces_nonzero_bytes() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    assert!(!buf.is_empty(), "binary save produced empty output");
    assert!(buf.len() > 100, "binary output suspiciously small: {} bytes", buf.len());
}

#[test]
fn binary_save_has_objects_section() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let found = buf.windows(b"Objects".len()).any(|w| w == b"Objects");
    assert!(found, "No 'Objects' section found in binary output");
}

#[test]
fn binary_save_has_connections_section() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let found = buf.windows(b"Connections".len()).any(|w| w == b"Connections");
    assert!(found, "No 'Connections' section found in binary output");
}

// ── Parseability ──────────────────────────────────────────────────────────────

#[test]
fn binary_save_is_parseable_by_loader() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(&buf);
    let result = FbxLoader.load(&mut cursor, &LoadOptions::default());
    assert!(result.is_ok(), "loader failed on binary output: {:?}", result.err());
}

#[test]
fn binary_save_empty_scene() {
    let scene = Scene::new();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    assert!(&buf[..23] == FBX_MAGIC.as_slice(), "magic missing for empty scene");

    let mut cursor = Cursor::new(&buf);
    let loaded = FbxLoader.load(&mut cursor, &LoadOptions::default()).unwrap();
    assert!(loaded.meshes.is_empty(), "empty scene should have no meshes");
}

// ── Round-trip: geometry ──────────────────────────────────────────────────────

#[test]
fn binary_round_trip_positions() {
    let scene  = triangle_scene();
    let loaded = binary_round_trip(&scene);
    assert!(!loaded.meshes.is_empty());
    let orig = &scene.meshes[0];
    let got  = &loaded.meshes[0];
    assert_eq!(orig.vertices.len(), got.vertices.len(), "vertex count changed");
    for (ov, lv) in orig.vertices.iter().zip(got.vertices.iter()) {
        assert!((ov.position.x - lv.position.x).abs() < 1e-4, "position.x mismatch");
        assert!((ov.position.y - lv.position.y).abs() < 1e-4, "position.y mismatch");
        assert!((ov.position.z - lv.position.z).abs() < 1e-4, "position.z mismatch");
    }
}

#[test]
fn binary_round_trip_normals() {
    let scene  = triangle_scene();
    let loaded = binary_round_trip(&scene);
    assert!(!loaded.meshes.is_empty());
    for v in &loaded.meshes[0].vertices {
        let n = v.normal.expect("normal missing after binary round-trip");
        assert!((n.z - 1.0).abs() < 1e-3, "normal.z should be ~1.0, got {}", n.z);
    }
}

#[test]
fn binary_round_trip_uvs() {
    let mut b    = SceneBuilder::new();
    let mut mesh = Mesh::new("UvMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new( 0.0,  1.0, 0.0)).with_normal(Vec3::Z).with_uv(Vec2::new(0.25, 0.5)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z).with_uv(Vec2::new(0.0,  0.5)),
        Vertex::new(Vec3::new( 1.0, -1.0, 0.0)).with_normal(Vec3::Z).with_uv(Vec2::new(0.75, 0.5)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi     = b.push_mesh(mesh);
    let r      = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene  = b.build();
    let loaded = binary_round_trip(&scene);

    let lm = &loaded.meshes[0];
    assert_eq!(lm.vertices.len(), 3, "vertex count changed");
    for v in &lm.vertices {
        assert!(v.uv().is_some(), "UV missing after binary round-trip");
    }
}

// ── Round-trip: counts ────────────────────────────────────────────────────────

#[test]
fn binary_round_trip_material_count() {
    let scene  = material_scene();
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        loaded.materials.len(), scene.materials.len(),
        "material count changed: {} vs {}", loaded.materials.len(), scene.materials.len()
    );
}

#[test]
fn binary_round_trip_node_count() {
    let mut b  = SceneBuilder::new();
    let r      = b.add_root_node("Root");
    let c      = b.add_child_node(r, "Child");
    let mi     = b.push_mesh(make_minimal_mesh("M"));
    b.attach_mesh(c, mi);
    let scene  = b.build();
    let loaded = binary_round_trip(&scene);
    assert!(
        loaded.nodes.len() >= 2,
        "expected ≥2 nodes, got {}", loaded.nodes.len()
    );
}

#[test]
fn binary_round_trip_mesh_name() {
    let mut b  = SceneBuilder::new();
    let mi     = b.push_mesh(make_minimal_mesh("NamedMesh"));
    let r      = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    let scene  = b.build();
    let loaded = binary_round_trip(&scene);
    assert!(!loaded.meshes.is_empty());
    assert_eq!(loaded.meshes[0].name, "NamedMesh", "mesh name not preserved");
}

// ── Multi-mesh ────────────────────────────────────────────────────────────────

#[test]
fn binary_save_multiple_meshes() {
    let mut b = SceneBuilder::new();
    let r     = b.add_root_node("Root");
    for i in 0..3 {
        let mi = b.push_mesh(make_minimal_mesh(&format!("M{i}")));
        let n  = b.add_child_node(r, format!("N{i}"));
        b.attach_mesh(n, mi);
    }
    let scene  = b.build();
    let loaded = binary_round_trip(&scene);
    assert_eq!(loaded.meshes.len(), 3, "expected 3 meshes, got {}", loaded.meshes.len());
}

// ── ASCII vs binary consistency ───────────────────────────────────────────────

#[test]
fn binary_save_and_ascii_save_same_mesh_count() {
    let mut b = SceneBuilder::new();
    let r     = b.add_root_node("Root");
    for i in 0..3 {
        let mi = b.push_mesh(make_minimal_mesh(&format!("M{i}")));
        let n  = b.add_child_node(r, format!("N{i}"));
        b.attach_mesh(n, mi);
    }
    let scene         = b.build();
    let loaded_ascii  = ascii_round_trip(&scene);
    let loaded_binary = binary_round_trip(&scene);
    assert_eq!(
        loaded_ascii.meshes.len(), loaded_binary.meshes.len(),
        "ASCII ({}) and binary ({}) mesh counts differ",
        loaded_ascii.meshes.len(), loaded_binary.meshes.len()
    );
}
