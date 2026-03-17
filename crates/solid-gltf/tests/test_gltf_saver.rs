//! Integration tests for GltfSaver.

mod common;

use common::*;
use solid_gltf::GltfSaver;
use solid_rs::prelude::*;

fn save_json(scene: &Scene) -> serde_json::Value {
    let mut buf = Vec::<u8>::new();
    GltfSaver.save(scene, &mut buf, &SaveOptions::default()).expect("save failed");
    serde_json::from_slice(&buf).expect("output is not valid JSON")
}

fn save_glb(scene: &Scene) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    GltfSaver.save_glb(scene, &mut buf).expect("save_glb failed");
    buf
}

// ── JSON validity ─────────────────────────────────────────────────────────────

#[test]
fn saver_produces_valid_json() {
    let json = save_json(&triangle_scene());
    // serde_json parsed it; if we got here it's valid JSON
    assert!(json.is_object());
}

#[test]
fn saver_asset_version_is_2_0() {
    let json = save_json(&triangle_scene());
    let version = json["asset"]["version"].as_str().unwrap();
    assert_eq!(version, "2.0");
}

// ── Accessors / buffer contents ───────────────────────────────────────────────

#[test]
fn saver_triangle_accessor_count() {
    let json = save_json(&triangle_scene());
    // At minimum: POSITION accessor + INDICES accessor
    let accessors = json["accessors"].as_array().unwrap();
    assert!(accessors.len() >= 2, "expected at least 2 accessors, got {}", accessors.len());
}

#[test]
fn saver_positions_in_buffer() {
    let json = save_json(&triangle_scene());
    // POSITION accessor must have count = 3
    let accessors = json["accessors"].as_array().unwrap();
    let pos_acc = accessors.iter().find(|a| {
        a["type"].as_str() == Some("VEC3")
            && a["count"].as_u64() == Some(3)
    });
    assert!(pos_acc.is_some(), "should have a VEC3 accessor with count=3 for positions");
}

#[test]
fn saver_normals_in_buffer() {
    let json = save_json(&triangle_scene());
    // Should have a NORMAL attribute in the primitive's attributes
    let prim = &json["meshes"][0]["primitives"][0];
    assert!(prim["attributes"]["NORMAL"].is_number(), "NORMAL accessor index should be present");
}

#[test]
fn saver_uvs_in_buffer() {
    let json = save_json(&pbr_material_scene());
    let prim = &json["meshes"][0]["primitives"][0];
    assert!(prim["attributes"]["TEXCOORD_0"].is_number(), "TEXCOORD_0 should be present");
}

// ── Material serialisation ────────────────────────────────────────────────────

#[test]
fn saver_material_base_color_factor() {
    let json = save_json(&pbr_material_scene());
    let factor = &json["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"];
    let arr = factor.as_array().unwrap();
    let r = arr[0].as_f64().unwrap() as f32;
    assert!((r - 0.8).abs() < 1e-5, "base color R should be ~0.8, got {r}");
}

#[test]
fn saver_material_roughness_factor() {
    let json = save_json(&pbr_material_scene());
    let roughness = json["materials"][0]["pbrMetallicRoughness"]["roughnessFactor"]
        .as_f64()
        .unwrap() as f32;
    assert!((roughness - 0.7).abs() < 1e-5, "roughnessFactor should be ~0.7, got {roughness}");
}

#[test]
fn saver_material_metallic_factor() {
    let json = save_json(&pbr_material_scene());
    let metallic = json["materials"][0]["pbrMetallicRoughness"]["metallicFactor"]
        .as_f64()
        .unwrap() as f32;
    assert!((metallic - 0.3).abs() < 1e-5, "metallicFactor should be ~0.3, got {metallic}");
}

#[test]
fn saver_material_specular_ior_extensions() {
    let json = save_json(&pbr_specular_ior_scene());

    let extensions_used = json["extensionsUsed"].as_array().unwrap();
    assert!(
        extensions_used.iter().any(|ext| ext.as_str() == Some("KHR_materials_specular")),
        "KHR_materials_specular should be advertised in extensionsUsed"
    );
    assert!(
        extensions_used.iter().any(|ext| ext.as_str() == Some("KHR_materials_ior")),
        "KHR_materials_ior should be advertised in extensionsUsed"
    );

    let specular = &json["materials"][0]["extensions"]["KHR_materials_specular"];
    assert!((specular["specularFactor"].as_f64().unwrap() as f32 - 0.65).abs() < 1e-5);
    let color = specular["specularColorFactor"].as_array().unwrap();
    assert!((color[0].as_f64().unwrap() as f32 - 0.9).abs() < 1e-5);
    assert!((color[1].as_f64().unwrap() as f32 - 0.8).abs() < 1e-5);
    assert!((color[2].as_f64().unwrap() as f32 - 0.7).abs() < 1e-5);

    let ior = json["materials"][0]["extensions"]["KHR_materials_ior"]["ior"]
        .as_f64()
        .unwrap() as f32;
    assert!((ior - 1.33).abs() < 1e-5);
}

#[test]
fn saver_material_alpha_mode_opaque() {
    let json = save_json(&pbr_material_scene());
    // Opaque is the default and should be omitted or null in JSON
    let alpha_mode = &json["materials"][0]["alphaMode"];
    // If absent (null/missing), it's implicitly OPAQUE per spec
    assert!(
        alpha_mode.is_null() || alpha_mode.as_str() == Some("OPAQUE"),
        "alpha mode should be absent or OPAQUE for opaque material, got: {alpha_mode}"
    );
}

#[test]
fn saver_material_alpha_mode_blend() {
    use solid_rs::builder::SceneBuilder;
    use solid_rs::scene::{AlphaMode, Material};
    let mut b = SceneBuilder::named("Blend");
    let mut mat = Material::new("BlendMat");
    mat.alpha_mode = AlphaMode::Blend;
    b.push_material(mat);
    b.add_root_node("R");
    let json = save_json(&b.build());
    assert_eq!(
        json["materials"][0]["alphaMode"].as_str().unwrap(),
        "BLEND"
    );
}

// ── Node structure ────────────────────────────────────────────────────────────

#[test]
fn saver_node_children_structure() {
    let json = save_json(&camera_scene());
    // Root node ("World") should list 2 children
    let nodes = json["nodes"].as_array().unwrap();
    let root_node = nodes.iter().find(|n| {
        n["children"].as_array().map_or(false, |c| c.len() == 2)
    });
    assert!(root_node.is_some(), "expected a node with 2 children");
}

// ── Cameras ───────────────────────────────────────────────────────────────────

#[test]
fn saver_camera_perspective() {
    let json = save_json(&camera_scene());
    let cameras = json["cameras"].as_array().unwrap();
    let persp = cameras.iter().find(|c| c["type"].as_str() == Some("perspective"));
    assert!(persp.is_some(), "expected a perspective camera in JSON");
    let yfov = persp.unwrap()["perspective"]["yfov"].as_f64().unwrap() as f32;
    assert!((yfov - 0.785398).abs() < 1e-4, "yfov mismatch: {yfov}");
}

#[test]
fn saver_camera_orthographic() {
    let json = save_json(&camera_scene());
    let cameras = json["cameras"].as_array().unwrap();
    let ortho = cameras.iter().find(|c| c["type"].as_str() == Some("orthographic"));
    assert!(ortho.is_some(), "expected an orthographic camera in JSON");
    let xmag = ortho.unwrap()["orthographic"]["xmag"].as_f64().unwrap() as f32;
    assert!((xmag - 5.0).abs() < 1e-5, "xmag mismatch: {xmag}");
}

// ── GLB binary container ──────────────────────────────────────────────────────

#[test]
fn saver_glb_magic_header() {
    let buf = save_glb(&triangle_scene());
    assert_eq!(&buf[0..4], b"glTF", "GLB magic bytes must be 'glTF'");
}

#[test]
fn saver_glb_json_chunk_present() {
    let buf = save_glb(&triangle_scene());
    // After 12-byte header: chunk0 length (4 bytes) + chunk type JSON = 0x4E4F534A
    assert!(buf.len() >= 20, "GLB too short");
    let chunk_type = u32::from_le_bytes(buf[16..20].try_into().unwrap());
    assert_eq!(chunk_type, 0x4E4F534A, "first chunk must be JSON (0x4E4F534A)");
}

#[test]
fn saver_glb_bin_chunk_present() {
    let buf = save_glb(&triangle_scene());
    // Parse past JSON chunk to reach BIN chunk
    let json_chunk_len = u32::from_le_bytes(buf[12..16].try_into().unwrap()) as usize;
    let bin_start = 12 + 8 + json_chunk_len;
    assert!(buf.len() > bin_start + 8, "GLB should have a BIN chunk");
    let bin_type = u32::from_le_bytes(buf[bin_start + 4..bin_start + 8].try_into().unwrap());
    assert_eq!(bin_type, 0x004E4942, "second chunk must be BIN (0x004E4942)");
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn saver_empty_scene_valid_json() {
    let scene = Scene::new();
    let json = save_json(&scene);
    assert_eq!(json["asset"]["version"].as_str().unwrap(), "2.0");
}

// ── Skin serialisation ────────────────────────────────────────────────────────

#[test]
fn saver_skin_joints_array() {
    let json = save_json(&skinned_scene());
    let skins = json["skins"].as_array().unwrap();
    assert!(!skins.is_empty(), "expected at least one skin");
    let joints = skins[0]["joints"].as_array().unwrap();
    assert_eq!(joints.len(), 2, "skin should have 2 joints");
}

// ── Animation serialisation ───────────────────────────────────────────────────

#[test]
fn saver_animation_channels_count() {
    let json = save_json(&animated_scene());
    let anims = json["animations"].as_array().unwrap();
    assert!(!anims.is_empty(), "expected at least one animation");
    let channels = anims[0]["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 2, "animation should have 2 channels");
}
