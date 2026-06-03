//! Tests for the USDA loader (parse from hand-crafted USDA strings).

mod common;

use glam::Vec3;
use solid_rs::prelude::LoadOptions;
use solid_rs::traits::Loader;
use solid_usd::UsdLoader;
use std::io::Cursor;

fn load_usda(src: &str) -> solid_rs::scene::Scene {
    UsdLoader
        .load(&mut Cursor::new(src.as_bytes()), &LoadOptions::default())
        .expect("load failed")
}

// ── Stage metadata ────────────────────────────────────────────────────────────

#[test]
fn loader_reads_up_axis() {
    let src = r#"#usda 1.0
( upAxis = "Y" )
def Xform "Root" {}
"#;
    let scene = load_usda(src);
    let up = scene.metadata.extra.get("upAxis");
    assert!(up.is_some(), "upAxis should be in scene metadata");
}

// ── Mesh geometry ─────────────────────────────────────────────────────────────

const TRIANGLE_USDA: &str = r#"#usda 1.0
(
    upAxis = "Y"
    defaultPrim = "Root"
)

def Xform "Root" {
    def Mesh "Triangle" {
        point3f[] points = [(0, 1, 0), (-1, -1, 0), (1, -1, 0)]
        int[] faceVertexCounts = [3]
        int[] faceVertexIndices = [0, 1, 2]
        normal3f[] normals = [(0, 0, 1), (0, 0, 1), (0, 0, 1)]
        uniform token subdivisionScheme = "none"
    }
}
"#;

#[test]
fn loader_triangle_vertex_count() {
    let scene = load_usda(TRIANGLE_USDA);
    assert!(!scene.meshes.is_empty(), "should have at least one mesh");
    assert_eq!(
        scene.meshes[0].vertices.len(),
        3,
        "triangle should have 3 vertices"
    );
}

#[test]
fn loader_triangle_positions() {
    let scene = load_usda(TRIANGLE_USDA);
    let positions: Vec<Vec3> = scene.meshes[0]
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    assert!((positions[0] - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-4);
    assert!((positions[1] - Vec3::new(-1.0, -1.0, 0.0)).length() < 1e-4);
    assert!((positions[2] - Vec3::new(1.0, -1.0, 0.0)).length() < 1e-4);
}

#[test]
fn loader_triangle_normals() {
    let scene = load_usda(TRIANGLE_USDA);
    for v in &scene.meshes[0].vertices {
        let n = v.normal.expect("normal should be present");
        assert!(
            (n - Vec3::Z).length() < 1e-4,
            "normal should be Vec3::Z, got {n:?}"
        );
    }
}

#[test]
fn loader_triangle_indices() {
    let scene = load_usda(TRIANGLE_USDA);
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

// ── Material ──────────────────────────────────────────────────────────────────

const MATERIAL_USDA: &str = r#"#usda 1.0
( defaultPrim = "Root" )

def Xform "Root" {
    def Material "RedMat" {
        rel outputs:surface = </Root/RedMat/RedMat_Shader.outputs:surface>
        def Shader "RedMat_Shader" {
            uniform token info:id = "UsdPreviewSurface"
            color3f inputs:diffuseColor = (0.8, 0.1, 0.1)
            float inputs:roughness = 0.5
            float inputs:metallic = 0.1
        }
    }
    def Mesh "Cube" {
        point3f[] points = [(0, 0, 0), (1, 0, 0), (1, 1, 0)]
        int[] faceVertexCounts = [3]
        int[] faceVertexIndices = [0, 1, 2]
        rel material:binding = </Root/RedMat>
        uniform token subdivisionScheme = "none"
    }
}
"#;

#[test]
fn loader_material_count() {
    let scene = load_usda(MATERIAL_USDA);
    assert_eq!(scene.materials.len(), 1, "should have one material");
}

#[test]
fn loader_material_base_color() {
    let scene = load_usda(MATERIAL_USDA);
    let c = scene.materials[0].base_color_factor;
    assert!((c.x - 0.8).abs() < 1e-4, "R mismatch: {}", c.x);
    assert!((c.y - 0.1).abs() < 1e-4, "G mismatch: {}", c.y);
}

#[test]
fn loader_material_roughness() {
    let scene = load_usda(MATERIAL_USDA);
    let r = scene.materials[0].roughness_factor;
    assert!((r - 0.5).abs() < 1e-4, "roughness mismatch: {r}");
}

#[test]
fn loader_material_binding() {
    let scene = load_usda(MATERIAL_USDA);
    // Material binding resolved means the mesh primitive has a material index
    assert!(
        scene.meshes[0].primitives[0].material_index.is_some(),
        "mesh should have a material binding",
    );
}

// ── Quad face triangulation ───────────────────────────────────────────────────

const QUAD_USDA: &str = r#"#usda 1.0
def Xform "Root" {
    def Mesh "Quad" {
        point3f[] points = [(0,0,0), (1,0,0), (1,1,0), (0,1,0)]
        int[] faceVertexCounts = [4]
        int[] faceVertexIndices = [0, 1, 2, 3]
        uniform token subdivisionScheme = "none"
    }
}
"#;

#[test]
fn loader_quad_triangulated_to_6_indices() {
    let scene = load_usda(QUAD_USDA);
    assert!(!scene.meshes.is_empty());
    // One quad = 2 triangles = 6 indices
    assert_eq!(
        scene.meshes[0].primitives[0].indices.len(),
        6,
        "quad should be fan-triangulated into 6 indices"
    );
}

// ── Reject binary USD ─────────────────────────────────────────────────────────

#[test]
fn loader_rejects_binary_usdc() {
    let fake_usdc = b"PXR-USDC\x00\x00\x00\x00";
    let result = UsdLoader.load(
        &mut Cursor::new(fake_usdc.as_ref()),
        &LoadOptions::default(),
    );
    assert!(result.is_err(), "binary USDC should be rejected");
}
