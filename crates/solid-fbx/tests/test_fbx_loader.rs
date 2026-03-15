//! Integration tests — `FbxLoader` acceptance, rejection, and basic parsing.

mod common;
use common::*;

use solid_fbx::{FbxLoader, FbxSaver};
use solid_rs::prelude::*;
use glam::*;
use std::io::Cursor;

// ── Rejection tests ───────────────────────────────────────────────────────────

#[test]
fn loader_rejects_empty_input() {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    let result = FbxLoader.load(&mut cursor, &LoadOptions::default());
    assert!(result.is_err(), "loader should reject empty input");
}

#[test]
fn loader_rejects_truncated_magic() {
    // Only the first 11 bytes of the binary magic — not enough.
    let mut cursor = Cursor::new(b"Kaydara FBX".to_vec());
    let result = FbxLoader.load(&mut cursor, &LoadOptions::default());
    assert!(result.is_err(), "loader should reject truncated magic");
}

#[test]
fn loader_rejects_wrong_magic() {
    let bad: Vec<u8> = b"This is definitely not an FBX file at all!".to_vec();
    let mut cursor   = Cursor::new(bad);
    let result       = FbxLoader.load(&mut cursor, &LoadOptions::default());
    assert!(result.is_err(), "loader should reject wrong magic");
}

// ── Acceptance tests ──────────────────────────────────────────────────────────

#[test]
fn loader_accepts_binary_produced_by_saver() {
    let scene   = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let result     = FbxLoader.load(&mut cursor, &LoadOptions::default());
    assert!(result.is_ok(), "loader rejected binary FBX produced by saver: {:?}", result.err());
}

#[test]
fn loader_accepts_ascii_produced_by_saver() {
    let scene   = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save(&scene, &mut buf, &SaveOptions::default()).unwrap();
    let mut cursor = Cursor::new(buf);
    let result     = FbxLoader.load(&mut cursor, &LoadOptions::default());
    assert!(result.is_ok(), "loader rejected ASCII FBX produced by saver: {:?}", result.err());
}

// ── Content tests ─────────────────────────────────────────────────────────────

#[test]
fn loader_empty_scene_has_no_meshes() {
    let scene   = Scene::new();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let loaded     = FbxLoader.load(&mut cursor, &LoadOptions::default()).unwrap();
    assert!(loaded.meshes.is_empty(),    "expected 0 meshes from empty scene");
    assert!(loaded.materials.is_empty(), "expected 0 materials from empty scene");
    assert!(loaded.lights.is_empty(),    "expected 0 lights from empty scene");
    assert!(loaded.cameras.is_empty(),   "expected 0 cameras from empty scene");
}

#[test]
fn loader_triangle_has_three_vertices() {
    let scene   = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let loaded     = FbxLoader.load(&mut cursor, &LoadOptions::default()).unwrap();
    assert!(!loaded.meshes.is_empty(), "no meshes");
    assert_eq!(
        loaded.meshes[0].vertices.len(), 3,
        "triangle should have 3 vertices, got {}", loaded.meshes[0].vertices.len()
    );
}

#[test]
fn loader_triangle_has_one_primitive() {
    let scene   = triangle_scene();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let loaded     = FbxLoader.load(&mut cursor, &LoadOptions::default()).unwrap();
    assert!(!loaded.meshes.is_empty(), "no meshes");
    assert_eq!(
        loaded.meshes[0].primitives.len(), 1,
        "triangle should have 1 primitive, got {}", loaded.meshes[0].primitives.len()
    );
}

#[test]
fn loader_material_count_correct() {
    let scene   = material_scene();
    let orig_mc = scene.materials.len();
    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let loaded     = FbxLoader.load(&mut cursor, &LoadOptions::default()).unwrap();
    assert_eq!(
        loaded.materials.len(), orig_mc,
        "material count: expected {}, got {}", orig_mc, loaded.materials.len()
    );
}

#[test]
fn loader_node_count_correct() {
    let mut b  = SceneBuilder::new();
    let r      = b.add_root_node("Root");
    let c      = b.add_child_node(r, "Child");
    let mi     = b.push_mesh(make_minimal_mesh("M"));
    b.attach_mesh(c, mi);
    let scene      = b.build();
    let orig_nc    = scene.nodes.len();
    let mut buf    = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let loaded     = FbxLoader.load(&mut cursor, &LoadOptions::default()).unwrap();
    assert!(
        loaded.nodes.len() >= orig_nc,
        "expected ≥{} nodes, got {}", orig_nc, loaded.nodes.len()
    );
}
