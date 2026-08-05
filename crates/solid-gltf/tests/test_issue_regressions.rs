//! Regression tests for glTF loader hardening (issues #10-#16, #30).
//!
//! Focus: malformed/hostile files must return `SolidError`, never panic, and
//! glTF `mode` / `normalized` / `extensionsRequired` semantics must be honored.

use solid_gltf::GltfLoader;
use solid_rs::prelude::*;
use std::io::Cursor;

fn load_bytes(bytes: &[u8]) -> solid_rs::Result<Scene> {
    GltfLoader.load(&mut Cursor::new(bytes), &LoadOptions::default())
}

/// Assemble a valid GLB from JSON + binary payload (both padded to 4 bytes).
fn make_glb(json: &str, bin: &[u8]) -> Vec<u8> {
    let pad = |b: &[u8], byte: u8| -> Vec<u8> {
        let mut v = b.to_vec();
        while v.len() % 4 != 0 {
            v.push(byte);
        }
        v
    };
    // GLB pads the JSON chunk with trailing space (0x20) characters.
    let json_padded = pad(json.as_bytes(), 0x20);
    let bin_padded = pad(bin, 0);
    let total = 12 + 8 + json_padded.len() + 8 + bin_padded.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_padded.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F534Au32.to_le_bytes());
    out.extend_from_slice(&json_padded);
    out.extend_from_slice(&(bin_padded.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E4942u32.to_le_bytes());
    out.extend_from_slice(&bin_padded);
    out
}

fn f32s(vals: &[f32]) -> Vec<u8> {
    vals.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn u32s(vals: &[u32]) -> Vec<u8> {
    vals.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// Base JSON: 3 vertices + 3 triangle indices in a GLB bin chunk.
fn base_json() -> String {
    r#"{
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": 48 }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 12 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0,0,0], "max": [1,1,0] },
            { "bufferView": 1, "componentType": 5125, "count": 3, "type": "SCALAR" }
        ],
        "meshes": [
            { "primitives": [ { "attributes": { "POSITION": 0 }, "indices": 1 } ] }
        ],
        "nodes": [ { "mesh": 0 } ],
        "scenes": [ { "nodes": [0] } ],
        "scene": 0
    }"#
    .to_string()
}

fn base_bin() -> Vec<u8> {
    // 3 vertices (0,0,0), (1,0,0), (0,1,0) + indices 0,1,2
    let mut b = f32s(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    b.extend(u32s(&[0, 1, 2]));
    b
}

#[test]
fn base_glb_loads() {
    let scene = load_bytes(&make_glb(&base_json(), &base_bin())).expect("base GLB should load");
    assert_eq!(scene.meshes.len(), 1);
    assert_eq!(scene.meshes[0].vertices.len(), 3);
    assert_eq!(scene.meshes[0].primitives[0].indices, vec![0, 1, 2]);
}

// ── #30: GLB chunk length slice panic ─────────────────────────────────────────

#[test]
fn glb_chunk_length_beyond_eof_errors_not_panics() {
    let mut glb = make_glb(&base_json(), &base_bin());
    // Overwrite the JSON chunk length field (bytes 12-15) with a huge value.
    glb[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    let err = load_bytes(&glb).expect_err("hostile chunk length must error, not panic");
    assert!(err.to_string().contains("chunk"), "unexpected error: {err}");
}

// ── #10: bufferView / accessor / image reference bounds ───────────────────────

#[test]
fn accessor_buffer_view_out_of_range_errors() {
    let json = base_json().replace("\"bufferView\": 0, \"componentType\": 5126", "\"bufferView\": 5, \"componentType\": 5126");
    let err = load_bytes(&make_glb(&json, &base_bin())).expect_err("bad bufferView index must error");
    assert!(err.to_string().contains("bufferView"), "unexpected error: {err}");
}

#[test]
fn accessor_index_out_of_range_errors() {
    let json = r#"{
        "asset": { "version": "2.0" },
        "animations": [
            { "channels": [ { "sampler": 0, "target": { "node": 0, "path": "translation" } } ],
              "samplers": [ { "input": 99, "output": 98, "interpolation": "LINEAR" } ] }
        ],
        "nodes": [ {} ],
        "scenes": [ { "nodes": [0] } ],
        "scene": 0
    }"#;
    let err = load_bytes(&make_glb(json, &[])).expect_err("out-of-range accessor index must error");
    assert!(err.to_string().contains("accessor"), "unexpected error: {err}");
}

#[test]
fn image_buffer_view_beyond_buffer_errors() {
    let json = r#"{
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": 48 }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 40, "byteLength": 12 }
        ],
        "images": [ { "bufferView": 1, "mimeType": "image/png" } ],
        "scenes": [ { "nodes": [] } ],
        "scene": 0
    }"#;
    let err = load_bytes(&make_glb(json, &base_bin())).expect_err("image view beyond buffer must error");
    assert!(err.to_string().contains("image"), "unexpected error: {err}");
}

// ── #11: accessor count/stride vs. buffer size ────────────────────────────────

#[test]
fn accessor_count_exceeding_buffer_errors() {
    // count 10 VEC3 needs 120 bytes but the view holds 36.
    let json = base_json().replace("\"count\": 3, \"type\": \"VEC3\"", "\"count\": 10, \"type\": \"VEC3\"");
    let err = load_bytes(&make_glb(&json, &base_bin())).expect_err("oversized accessor must error");
    assert!(err.to_string().contains("bytes"), "unexpected error: {err}");
}

#[test]
fn hostile_accessor_count_is_rejected_before_allocation() {
    // count ~1e9 * 12 bytes = ~12 GiB: must error fast, not OOM.
    let json = base_json().replace("\"count\": 3, \"type\": \"VEC3\"", "\"count\": 1000000000, \"type\": \"VEC3\"");
    let err = load_bytes(&make_glb(&json, &base_bin())).expect_err("hostile count must error");
    assert!(err.to_string().contains("limit"), "unexpected error: {err}");
}

// ── #12: bufferView.byte_length must bound accessor reads ─────────────────────

#[test]
fn buffer_view_byte_length_limits_accessor_reads() {
    // View 0 declares 24 bytes, but 3 VEC3s need 36 -> must error.
    let json = base_json().replace("\"byteOffset\": 0,  \"byteLength\": 36", "\"byteOffset\": 0,  \"byteLength\": 24");
    let err = load_bytes(&make_glb(&json, &base_bin())).expect_err("byte_length shortfall must error");
    assert!(err.to_string().contains("bytes"), "unexpected error: {err}");
}

// ── #13: primitive.mode handling ──────────────────────────────────────────────

#[test]
fn primitive_mode_triangle_strip_is_expanded() {
    // Strip of 4 vertices -> (0,1,2) and (2,1,3) [flip on odd step].
    let mut bin = f32s(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0]);
    bin.extend(u32s(&[0, 1, 2, 3]));
    let json = r#"{
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": 64 }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0,  "byteLength": 48 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 16 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
              "min": [0,0,0], "max": [1,1,0] },
            { "bufferView": 1, "componentType": 5125, "count": 4, "type": "SCALAR" }
        ],
        "meshes": [
            { "primitives": [ { "attributes": { "POSITION": 0 }, "indices": 1, "mode": 5 } ] }
        ],
        "nodes": [ { "mesh": 0 } ],
        "scenes": [ { "nodes": [0] } ],
        "scene": 0
    }"#;
    let scene = load_bytes(&make_glb(json, &bin)).expect("strip should load");
    assert_eq!(
        scene.meshes[0].primitives[0].indices,
        vec![0, 1, 2, 1, 3, 2],
        "strip must be expanded to triangles with correct winding"
    );
}

#[test]
fn primitive_mode_triangle_fan_is_expanded() {
    // Fan of 4 vertices -> (0,1,2) and (0,2,3).
    let mut bin = f32s(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0]);
    bin.extend(u32s(&[0, 1, 2, 3]));
    let json = r#"{
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": 64 }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0,  "byteLength": 48 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 16 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
              "min": [0,0,0], "max": [1,1,0] },
            { "bufferView": 1, "componentType": 5125, "count": 4, "type": "SCALAR" }
        ],
        "meshes": [
            { "primitives": [ { "attributes": { "POSITION": 0 }, "indices": 1, "mode": 6 } ] }
        ],
        "nodes": [ { "mesh": 0 } ],
        "scenes": [ { "nodes": [0] } ],
        "scene": 0
    }"#;
    let scene = load_bytes(&make_glb(json, &bin)).expect("fan should load");
    assert_eq!(scene.meshes[0].primitives[0].indices, vec![0, 1, 2, 0, 2, 3]);
}

#[test]
fn primitive_mode_lines_is_rejected() {
    let json = base_json().replace("\"indices\": 1 } ] }", "\"indices\": 1, \"mode\": 1 } ] }");
    let err = load_bytes(&make_glb(&json, &base_bin())).expect_err("LINES mode must be rejected");
    assert!(err.to_string().contains("mode"), "unexpected error: {err}");
}

// ── #14: index accessor values validated against vertex count ─────────────────

#[test]
fn out_of_range_index_value_errors() {
    let mut bin = f32s(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    bin.extend(u32s(&[0, 1, 99]));
    let err = load_bytes(&make_glb(&base_json(), &bin)).expect_err("OOB index must error");
    assert!(err.to_string().contains("out of range"), "unexpected error: {err}");
}

// ── #15: normalized signed integer accessors ──────────────────────────────────

#[test]
fn normalized_signed_byte_colors_decode() {
    // 3 vertices + COLOR_0 as VEC4 normalized i8.
    let mut bin = f32s(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    bin.extend(u32s(&[0, 1, 2]));
    // 3 x VEC4 i8: v0=(-128,-1,127,127) v1=(-1,0,0,127) v2=(127,0,0,127)
    bin.extend_from_slice(&[0x80, 0xFF, 0x7F, 0x7F]);
    bin.extend_from_slice(&[0xFF, 0x00, 0x00, 0x7F]);
    bin.extend_from_slice(&[0x7F, 0x00, 0x00, 0x7F]);
    let json = r#"{
        "asset": { "version": "2.0" },
        "buffers": [{ "byteLength": 60 }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 12 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 12 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0,0,0], "max": [1,1,0] },
            { "bufferView": 1, "componentType": 5125, "count": 3, "type": "SCALAR" },
            { "bufferView": 2, "componentType": 5120, "count": 3, "type": "VEC4", "normalized": true }
        ],
        "meshes": [
            { "primitives": [ { "attributes": { "POSITION": 0, "COLOR_0": 2 }, "indices": 1 } ] }
        ],
        "nodes": [ { "mesh": 0 } ],
        "scenes": [ { "nodes": [0] } ],
        "scene": 0
    }"#;
    let scene = load_bytes(&make_glb(json, &bin)).expect("normalized i8 should load");
    let colors: Vec<f32> = scene.meshes[0]
        .vertices
        .iter()
        .map(|v| v.colors[0].map(|c| c.x).unwrap_or(0.0))
        .collect();
    assert!((colors[0] - (-128.0 / 127.0)).abs() < 1e-4, "colors[0]={}", colors[0]);
    assert!((colors[1] - (-1.0 / 127.0)).abs() < 1e-4, "colors[1]={}", colors[1]);
    assert!((colors[2] - 1.0).abs() < 1e-4, "colors[2]={}", colors[2]);
}

// ── #16: extensionsRequired enforcement ───────────────────────────────────────

#[test]
fn draco_extensions_required_is_rejected() {
    let json = base_json().replace(
        "\"asset\": { \"version\": \"2.0\" }",
        "\"asset\": { \"version\": \"2.0\" },\n        \"extensionsRequired\": [\"KHR_draco_mesh_compression\"]",
    );
    let err = load_bytes(&make_glb(&json, &base_bin())).expect_err("draco-required must error");
    assert!(err.to_string().contains("KHR_draco_mesh_compression"), "unexpected error: {err}");
}

#[test]
fn supported_extensions_required_is_accepted() {
    let json = base_json().replace(
        "\"asset\": { \"version\": \"2.0\" }",
        "\"asset\": { \"version\": \"2.0\" },\n        \"extensionsRequired\": [\"KHR_materials_specular\"]",
    );
    load_bytes(&make_glb(&json, &base_bin())).expect("supported required extension should load");
}
