//! `KHR_texture_transform`: each texture slot keeps its own transform and UV
//! set, files that require the extension load, and the saver writes it back.

mod common;

use common::*;
use glam::Vec2;
use solid_gltf::{GltfLoader, GltfSaver};
use solid_rs::prelude::*;
use solid_rs::scene::TextureTransform;
use std::io::Cursor;

const FIXTURE: &str = include_str!("fixtures/texture_transform.gltf");

fn load_fixture() -> Scene {
    GltfLoader
        .load(
            &mut Cursor::new(FIXTURE.as_bytes()),
            &LoadOptions::default(),
        )
        .expect("a glTF requiring KHR_texture_transform should load")
}

fn transform(offset: [f32; 2], rotation: f32, scale: [f32; 2]) -> TextureTransform {
    TextureTransform {
        offset: Vec2::from(offset),
        rotation,
        scale: Vec2::from(scale),
    }
}

#[test]
fn required_texture_transform_loads() {
    let scene = load_fixture();
    assert_eq!(scene.materials.len(), 1);
    assert_eq!(scene.meshes.len(), 1);
}

#[test]
fn offset_and_scale_carry_onto_the_slot() {
    let scene = load_fixture();
    let base = scene.materials[0].base_color_texture.as_ref().unwrap();
    assert_eq!(base.texture_index, 0);
    assert_eq!(base.uv_channel, 0);
    assert_eq!(
        base.transform,
        Some(transform([0.25, 0.5], 0.0, [2.0, 4.0]))
    );
}

#[test]
fn tex_coord_override_picks_the_uv_set() {
    let scene = load_fixture();
    let mr = scene.materials[0]
        .metallic_roughness_texture
        .as_ref()
        .unwrap();
    assert_eq!(
        mr.uv_channel, 1,
        "the extension's texCoord overrides the slot's"
    );
    assert_eq!(mr.transform, Some(transform([0.0, 0.0], 0.0, [3.0, 3.0])));
}

#[test]
fn rotation_carries_verbatim() {
    let scene = load_fixture();
    let normal = scene.materials[0].normal_texture.as_ref().unwrap();
    assert_eq!(normal.uv_channel, 0);
    assert_eq!(
        normal.transform,
        Some(transform([0.0, 0.0], 1.570_796_4, [1.0, 1.0]))
    );
    assert_eq!(scene.materials[0].normal_scale, 0.5);
}

#[test]
fn slots_without_a_transform_stay_plain() {
    let scene = load_fixture();
    let material = &scene.materials[0];
    let occlusion = material.occlusion_texture.as_ref().unwrap();
    assert_eq!(occlusion.uv_channel, 1);
    assert_eq!(occlusion.transform, None);
    assert_eq!(material.occlusion_strength, 0.75);
    // An explicit identity transform is no transform.
    assert_eq!(material.emissive_texture.as_ref().unwrap().transform, None);
}

#[test]
fn extension_texture_slots_carry_their_transform() {
    let scene = load_fixture();
    let specular = scene.materials[0].specular_weight_texture.as_ref().unwrap();
    assert_eq!(specular.texture_index, 2);
    assert_eq!(
        specular.transform,
        Some(transform([-0.5, 0.125], 0.0, [1.0, 1.0]))
    );
}

#[test]
fn vertex_uvs_are_not_baked() {
    // Base colour and normal disagree on UV set 0; both keep their own
    // transform, so the vertices keep the authored UVs.
    let scene = load_fixture();
    let uv0: Vec<Vec2> = scene.meshes[0]
        .vertices
        .iter()
        .map(|v| v.uvs[0].unwrap())
        .collect();
    let uv1: Vec<Vec2> = scene.meshes[0]
        .vertices
        .iter()
        .map(|v| v.uvs[1].unwrap())
        .collect();
    assert_eq!(
        uv0,
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0)
        ]
    );
    assert_eq!(
        uv1,
        [
            Vec2::new(0.5, 0.5),
            Vec2::new(1.0, 0.5),
            Vec2::new(0.5, 1.0)
        ]
    );
}

#[test]
fn saver_writes_the_extension_and_reloads_it() {
    let scene = load_fixture();
    let mut out = Vec::new();
    GltfSaver
        .save(&scene, &mut out, &SaveOptions::default())
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let used = json["extensionsUsed"].as_array().unwrap();
    assert!(used.iter().any(|e| e == "KHR_texture_transform"));
    // The transform is optional for a reader: used, never required.
    assert!(json["extensionsRequired"]
        .as_array()
        .is_none_or(|r| !r.iter().any(|e| e == "KHR_texture_transform")));
    let base = &json["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"];
    assert_eq!(
        base["extensions"]["KHR_texture_transform"],
        serde_json::json!({ "offset": [0.25, 0.5], "scale": [2.0, 4.0] })
    );
    // A slot without a transform writes no extensions object.
    assert!(json["materials"][0]["occlusionTexture"]
        .get("extensions")
        .is_none());

    let reloaded = gltf_round_trip(&scene);
    let (a, b) = (&scene.materials[0], &reloaded.materials[0]);
    for (before, after) in [
        (&a.base_color_texture, &b.base_color_texture),
        (&a.metallic_roughness_texture, &b.metallic_roughness_texture),
        (&a.normal_texture, &b.normal_texture),
        (&a.occlusion_texture, &b.occlusion_texture),
        (&a.emissive_texture, &b.emissive_texture),
        (&a.specular_weight_texture, &b.specular_weight_texture),
    ] {
        let (before, after) = (before.as_ref().unwrap(), after.as_ref().unwrap());
        assert_eq!(before.uv_channel, after.uv_channel);
        assert_eq!(before.transform, after.transform);
    }
}

#[test]
fn scene_without_transforms_does_not_declare_the_extension() {
    let mut out = Vec::new();
    GltfSaver
        .save(&pbr_material_scene(), &mut out, &SaveOptions::default())
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(json["extensionsUsed"]
        .as_array()
        .is_none_or(|u| !u.iter().any(|e| e == "KHR_texture_transform")));
}
