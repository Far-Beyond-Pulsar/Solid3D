//! Integration tests for StlLoader.

mod common;

use common::*;
use solid_rs::prelude::*;
use solid_stl::StlLoader;
use std::io::Cursor;

// ── Detection ─────────────────────────────────────────────────────────────────

#[test]
fn loader_detects_binary() {
    // A correctly-sized binary STL must be loaded successfully (binary path).
    let data = single_triangle_binary();
    let scene = StlLoader
        .load(&mut Cursor::new(data), &LoadOptions::default())
        .expect("binary STL should load");
    // Exactly 3 unique vertices → one triangle
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

#[test]
fn loader_detects_ascii() {
    // An ASCII STL must be loaded successfully (ASCII path).
    let data = single_triangle_ascii();
    let scene = StlLoader
        .load(&mut Cursor::new(data), &LoadOptions::default())
        .expect("ASCII STL should load");
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

// ── Binary – single triangle ──────────────────────────────────────────────────

#[test]
fn loader_binary_single_triangle_vertex_count() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(single_triangle_binary()),
            &LoadOptions::default(),
        )
        .unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_binary_single_triangle_index_count() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(single_triangle_binary()),
            &LoadOptions::default(),
        )
        .unwrap();
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

#[test]
fn loader_binary_positions_correct() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(single_triangle_binary()),
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

// ── Binary – two triangles ────────────────────────────────────────────────────

#[test]
fn loader_binary_two_triangles() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(two_triangle_binary()),
            &LoadOptions::default(),
        )
        .unwrap();
    // Quad split into 2 tris → 4 unique vertices, 6 indices
    assert_eq!(scene.meshes[0].vertices.len(), 4);
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 6);
}

// ── Binary – zero triangles ───────────────────────────────────────────────────

#[test]
fn loader_binary_zero_triangles() {
    let mut buf = vec![0u8; 84]; // 80-byte header + 4-byte count = 0
                                 // count stays 0 (all zeros)
    let scene = StlLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .expect("empty binary STL should not panic");
    // One mesh is always created; it just has no data
    assert!(scene.meshes[0].vertices.is_empty());
}

// ── ASCII ─────────────────────────────────────────────────────────────────────

#[test]
fn loader_ascii_single_triangle_vertex_count() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(single_triangle_ascii()),
            &LoadOptions::default(),
        )
        .unwrap();
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_ascii_positions_correct() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(single_triangle_ascii()),
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
fn loader_ascii_solid_name_used() {
    let ascii = b"solid MyModel\n\
      facet normal 0 0 1\n\
        outer loop\n\
          vertex 0 0 0\n\
          vertex 1 0 0\n\
          vertex 0 1 0\n\
        endloop\n\
      endfacet\n\
    endsolid MyModel\n";
    let scene = StlLoader
        .load(&mut Cursor::new(ascii.to_vec()), &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].name, "MyModel");
}

#[test]
fn loader_ascii_multiple_solids() {
    // Two solid blocks → parser collects all triangles into one mesh.
    let ascii = b"solid First\n\
      facet normal 0 0 1\n\
        outer loop\n\
          vertex 0 0 0\n\
          vertex 1 0 0\n\
          vertex 0 1 0\n\
        endloop\n\
      endfacet\n\
    endsolid First\n\
    solid Second\n\
      facet normal 0 0 1\n\
        outer loop\n\
          vertex 2 0 0\n\
          vertex 3 0 0\n\
          vertex 2 1 0\n\
        endloop\n\
      endfacet\n\
    endsolid Second\n";
    let scene = StlLoader
        .load(&mut Cursor::new(ascii.to_vec()), &LoadOptions::default())
        .unwrap();
    // Both triangles are loaded into the single mesh.
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 6);
}

// ── VisCAM color ──────────────────────────────────────────────────────────────

#[test]
fn loader_viscam_color_red_loaded() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(colored_triangle_binary(31, 0, 0)),
            &LoadOptions::default(),
        )
        .unwrap();
    let v = &scene.meshes[0].vertices[0];
    let c = v.colors[0].expect("red vertex should have a color");
    assert!((c.x - 1.0).abs() < 1e-4, "R should be 1.0, got {}", c.x);
    assert!(c.y < 1e-4, "G should be 0.0, got {}", c.y);
    assert!(c.z < 1e-4, "B should be 0.0, got {}", c.z);
}

#[test]
fn loader_viscam_color_green_loaded() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(colored_triangle_binary(0, 31, 0)),
            &LoadOptions::default(),
        )
        .unwrap();
    let v = &scene.meshes[0].vertices[0];
    let c = v.colors[0].expect("green vertex should have a color");
    assert!(c.x < 1e-4, "R should be 0.0");
    assert!((c.y - 1.0).abs() < 1e-4, "G should be 1.0");
    assert!(c.z < 1e-4, "B should be 0.0");
}

#[test]
fn loader_viscam_color_white_loaded() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(colored_triangle_binary(31, 31, 31)),
            &LoadOptions::default(),
        )
        .unwrap();
    let v = &scene.meshes[0].vertices[0];
    let c = v.colors[0].expect("white vertex should have a color");
    assert!((c.x - 1.0).abs() < 1e-4, "R should be 1.0");
    assert!((c.y - 1.0).abs() < 1e-4, "G should be 1.0");
    assert!((c.z - 1.0).abs() < 1e-4, "B should be 1.0");
}

#[test]
fn loader_no_viscam_color_when_bit15_clear() {
    // attr = 0x1234: bit 15 is 0, so no color should be set.
    let mut data = single_triangle_binary();
    let attr_off = 84 + 48; // header(80) + count(4) + 1 triangle * 48 bytes
    data[attr_off] = 0x34;
    data[attr_off + 1] = 0x12; // LE 0x1234, bit-15 = 0
    let scene = StlLoader
        .load(&mut Cursor::new(data), &LoadOptions::default())
        .unwrap();
    for v in &scene.meshes[0].vertices {
        assert!(
            v.colors[0].is_none(),
            "no color expected when bit-15 is clear"
        );
    }
}

// ── Smooth normals ────────────────────────────────────────────────────────────

#[test]
fn loader_smooth_normals_computed() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(single_triangle_binary()),
            &LoadOptions::default(),
        )
        .unwrap();
    // Every vertex in a triangle mesh should have a computed smooth normal.
    for v in &scene.meshes[0].vertices {
        assert!(
            v.normal.is_some(),
            "smooth normal should be set on every vertex"
        );
    }
}

#[test]
fn loader_smooth_normals_unit_length() {
    let scene = StlLoader
        .load(
            &mut Cursor::new(two_triangle_binary()),
            &LoadOptions::default(),
        )
        .unwrap();
    for v in &scene.meshes[0].vertices {
        if let Some(n) = v.normal {
            let len = n.length();
            assert!(
                (len - 1.0).abs() < 1e-5,
                "normal length should be 1.0, got {len}"
            );
        }
    }
}

// ── Error handling ────────────────────────────────────────────────────────────

#[test]
fn loader_rejects_truncated_binary() {
    // Use 0xFF bytes so ASCII parsing fails (invalid UTF-8), forcing the
    // binary fallback path which will detect truncation.
    // detect_binary: expected=84+100*50=5084 ≠ 84 → false
    // parse_ascii:   invalid UTF-8 → Err
    // parse_binary (fallback): data.len() 84 < 5084 → "file truncated" Err
    let mut buf = vec![0xFFu8; 84];
    buf[80] = 100; // count = 100 (LE) → needs 5084 bytes
    buf[81] = 0;
    buf[82] = 0;
    buf[83] = 0;
    let result = StlLoader.load(&mut Cursor::new(buf), &LoadOptions::default());
    assert!(result.is_err(), "truncated binary STL should be rejected");
}
