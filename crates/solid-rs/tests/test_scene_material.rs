mod common;
use glam::{Vec3, Vec4};
use solid_rs::prelude::*;

// ── Material::default ────────────────────────────────────────────────────────

#[test]
fn material_default_base_color_is_white() {
    assert_eq!(Material::default().base_color_factor, Vec4::ONE);
}
#[test]
fn material_default_metallic_is_1() {
    assert_eq!(Material::default().metallic_factor, 1.0);
}
#[test]
fn material_default_roughness_is_1() {
    assert_eq!(Material::default().roughness_factor, 1.0);
}
#[test]
fn material_default_emissive_is_black() {
    assert_eq!(Material::default().emissive_factor, Vec3::ZERO);
}
#[test]
fn material_default_alpha_is_opaque() {
    assert_eq!(Material::default().alpha_mode, AlphaMode::Opaque);
}
#[test]
fn material_default_alpha_cutoff_is_half() {
    assert_eq!(Material::default().alpha_cutoff, 0.5);
}
#[test]
fn material_default_not_double_sided() {
    assert!(!Material::default().double_sided);
}
#[test]
fn material_default_no_base_tex() {
    assert!(Material::default().base_color_texture.is_none());
}
#[test]
fn material_default_no_normal_tex() {
    assert!(Material::default().normal_texture.is_none());
}
#[test]
fn material_default_no_emissive_tex() {
    assert!(Material::default().emissive_texture.is_none());
}
#[test]
fn material_default_normal_scale_1() {
    assert_eq!(Material::default().normal_scale, 1.0);
}
#[test]
fn material_default_occlusion_strength_1() {
    assert_eq!(Material::default().occlusion_strength, 1.0);
}

// ── Material::new ─────────────────────────────────────────────────────────────

#[test]
fn material_new_sets_name() {
    let m = Material::new("PBR");
    assert_eq!(m.name, "PBR");
}

#[test]
fn material_new_empty_string_name() {
    let m = Material::new("");
    assert_eq!(m.name, "");
}

// ── Material::solid_color ─────────────────────────────────────────────────────

#[test]
fn material_solid_color_sets_base_color() {
    let c = Vec4::new(0.8, 0.2, 0.1, 1.0);
    let m = Material::solid_color("Red", c);
    assert_eq!(m.base_color_factor, c);
}

#[test]
fn material_solid_color_sets_name() {
    let m = Material::solid_color("Green", Vec4::new(0.0, 1.0, 0.0, 1.0));
    assert_eq!(m.name, "Green");
}

#[test]
fn material_solid_color_no_texture() {
    let m = Material::solid_color("X", Vec4::ONE);
    assert!(m.base_color_texture.is_none());
}

// ── AlphaMode ────────────────────────────────────────────────────────────────

#[test]
fn alpha_mode_default_is_opaque() {
    assert_eq!(AlphaMode::default(), AlphaMode::Opaque);
}
#[test]
fn alpha_mode_mask_neq_opaque() {
    assert_ne!(AlphaMode::Mask, AlphaMode::Opaque);
}
#[test]
fn alpha_mode_blend_neq_opaque() {
    assert_ne!(AlphaMode::Blend, AlphaMode::Opaque);
}
#[test]
fn alpha_mode_copy() {
    let a = AlphaMode::Mask;
    let b = a;
    assert_eq!(a, b);
}

// ── TextureRef ────────────────────────────────────────────────────────────────

#[test]
fn texture_ref_new_index() {
    let r = TextureRef::new(3);
    assert_eq!(r.texture_index, 3);
}

#[test]
fn texture_ref_new_default_uv_channel() {
    let r = TextureRef::new(0);
    assert_eq!(r.uv_channel, 0);
}

#[test]
fn texture_ref_new_no_transform() {
    let r = TextureRef::new(0);
    assert!(r.transform.is_none());
}

#[test]
fn texture_ref_partial_eq() {
    let a = TextureRef::new(2);
    let b = TextureRef::new(2);
    assert_eq!(a, b);
}

#[test]
fn texture_ref_diff_index_not_eq() {
    let a = TextureRef::new(0);
    let b = TextureRef::new(1);
    assert_ne!(a, b);
}

// ── TextureTransform ──────────────────────────────────────────────────────────

#[test]
fn texture_transform_default_offset_zero() {
    use glam::Vec2;
    assert_eq!(TextureTransform::default().offset, Vec2::ZERO);
}

#[test]
fn texture_transform_default_scale_one() {
    use glam::Vec2;
    assert_eq!(TextureTransform::default().scale, Vec2::ONE);
}

#[test]
fn texture_transform_default_rotation_zero() {
    assert_eq!(TextureTransform::default().rotation, 0.0);
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn material_clone_preserves_name() {
    let m = Material::new("Clone Me");
    assert_eq!(m.clone().name, "Clone Me");
}

#[test]
fn material_clone_preserves_metallic() {
    let mut m = Material::new("X");
    m.metallic_factor = 0.42;
    assert!((m.clone().metallic_factor - 0.42).abs() < 1e-6);
}

// ── Field mutation ────────────────────────────────────────────────────────────

#[test]
fn material_set_double_sided() {
    let mut m = Material::new("X");
    m.double_sided = true;
    assert!(m.double_sided);
}

#[test]
fn material_set_alpha_mask() {
    let mut m = Material::new("X");
    m.alpha_mode = AlphaMode::Mask;
    m.alpha_cutoff = 0.3;
    assert_eq!(m.alpha_mode, AlphaMode::Mask);
    assert!((m.alpha_cutoff - 0.3).abs() < 1e-6);
}

#[test]
fn material_set_emissive() {
    let mut m = Material::new("Glow");
    m.emissive_factor = Vec3::new(5.0, 4.0, 0.0);
    assert_eq!(m.emissive_factor, Vec3::new(5.0, 4.0, 0.0));
}

// ── Extensions ────────────────────────────────────────────────────────────────

#[test]
fn material_extensions_initially_empty() {
    assert!(Material::new("X").extensions.is_empty());
}

#[test]
fn material_extensions_store_custom() {
    #[derive(Debug)]
    struct FbxProps {
        id: u32,
    }
    let mut m = Material::new("X");
    m.extensions.insert(FbxProps { id: 99 });
    assert_eq!(m.extensions.get::<FbxProps>().unwrap().id, 99);
}
