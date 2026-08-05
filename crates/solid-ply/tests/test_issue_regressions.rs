//! Regression tests for bugs filed against solid-ply:
//!   #23  Face vertex indices are not validated against the vertex count
//!   #24  Element/face count mismatches are silently accepted
//!   #25  Binary cursor advances past the end of the body unchecked
//!   #26  Hostile element counts cause huge allocations / CPU loops

use solid_ply::PlyLoader;
use solid_rs::prelude::*;
use std::io::Cursor;

const ASCII_HEADER3: &str = "\
ply\n\
format ascii 1.0\n\
element vertex 3\n\
property float x\n\
property float y\n\
property float z\n\
element face 1\n\
property list uchar uint vertex_indices\n\
end_header\n";

fn load_bytes(data: &[u8]) -> Result<Scene, solid_rs::SolidError> {
    PlyLoader.load(&mut Cursor::new(data.to_vec()), &LoadOptions::default())
}

// ── #23  Face index validation ────────────────────────────────────────────────

#[test]
fn ascii_face_out_of_range_index_errors() {
    let ply = format!("{ASCII_HEADER3}0 0 0\n1 0 0\n0 1 0\n3 0 1 9\n");
    let err = load_bytes(ply.as_bytes()).unwrap_err();
    assert!(
        format!("{err:?}").contains("out of range"),
        "expected out-of-range error, got {err:?}"
    );
}

fn binary_ply_face(indices: &[u32], truncated_quality: bool) -> Vec<u8> {
    let header = b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nproperty float quality\nend_header\n";
    let mut out = header.to_vec();
    // vertex: (0,0,0)
    out.extend_from_slice(&0.0f32.to_le_bytes());
    out.extend_from_slice(&0.0f32.to_le_bytes());
    out.extend_from_slice(&0.0f32.to_le_bytes());
    // face: count, indices, trailing scalar "quality"
    out.push(indices.len() as u8);
    for &i in indices {
        out.extend_from_slice(&i.to_le_bytes());
    }
    if !truncated_quality {
        out.extend_from_slice(&1.0f32.to_le_bytes());
    }
    out
}

#[test]
fn binary_face_out_of_range_index_errors() {
    let bytes = binary_ply_face(&[0, 0, 9], false);
    let err = load_bytes(&bytes).unwrap_err();
    assert!(
        format!("{err:?}").contains("out of range"),
        "expected out-of-range error, got {err:?}"
    );
}

// ── #24  Count mismatch / truncation ──────────────────────────────────────────

#[test]
fn ascii_face_count_mismatch_errors() {
    // Declares 4 vertices but provides only 3.
    let ply = format!("{ASCII_HEADER3}0 0 0\n1 0 0\n0 1 0\n4 0 1 2\n");
    let err = load_bytes(ply.as_bytes()).unwrap_err();
    assert!(
        format!("{err:?}").contains("declares 4"),
        "expected count-mismatch error, got {err:?}"
    );
}

#[test]
fn ascii_vertex_truncation_errors() {
    // Declares 3 vertices but only 2 lines are present (the third "row" is the
    // face line, so the face element runs out of input).
    let ply = format!("{ASCII_HEADER3}0 0 0\n1 0 0\n3 0 1 2\n");
    let err = load_bytes(ply.as_bytes()).unwrap_err();
    assert!(
        matches!(err, SolidError::Parse(_)),
        "expected a parse error for truncated body, got {err:?}"
    );
}

// ── #25  Binary cursor bounds ─────────────────────────────────────────────────

#[test]
fn binary_truncated_face_scalar_errors() {
    // The "quality" scalar after the face list is missing — the cursor must
    // not silently advance past the end of the body.
    let bytes = binary_ply_face(&[0], true);
    let err = load_bytes(&bytes).unwrap_err();
    assert!(
        format!("{err:?}").contains("unexpected end of binary"),
        "expected truncation error, got {err:?}"
    );
}

// ── #26  Hostile element counts ───────────────────────────────────────────────

#[test]
fn ascii_huge_vertex_count_is_rejected_before_allocating() {
    let ply = "ply\nformat ascii 1.0\nelement vertex 4294967295\nproperty float x\nproperty float y\nproperty float z\nend_header\n0 0 0\n";
    let err = load_bytes(ply.as_bytes()).unwrap_err();
    assert!(
        format!("{err:?}").contains("declares 4294967295"),
        "expected hostile-count error, got {err:?}"
    );
}

#[test]
fn binary_huge_vertex_count_is_rejected_before_allocating() {
    let header = b"ply\nformat binary_little_endian 1.0\nelement vertex 4294967295\nproperty float x\nproperty float y\nproperty float z\nend_header\n";
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(&[0u8; 12]);
    let err = load_bytes(&bytes).unwrap_err();
    assert!(
        format!("{err:?}").contains("declares 4294967295"),
        "expected hostile-count error, got {err:?}"
    );
}

// Valid control — the well-formed binary file still loads.
#[test]
fn binary_valid_face_with_quality_loads() {
    let bytes = binary_ply_face(&[0, 0, 0], false);
    let scene = load_bytes(&bytes).unwrap();
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}
