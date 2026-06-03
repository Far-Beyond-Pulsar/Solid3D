mod common;
use glam::{Mat4, Quat, Vec3};
use solid_rs::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

// ── IDENTITY constant ─────────────────────────────────────────────────────────

#[test]
fn transform_identity_translation_is_zero() {
    assert_eq!(Transform::IDENTITY.translation, Vec3::ZERO);
}

#[test]
fn transform_identity_rotation_is_identity() {
    assert_eq!(Transform::IDENTITY.rotation, Quat::IDENTITY);
}

#[test]
fn transform_identity_scale_is_one() {
    assert_eq!(Transform::IDENTITY.scale, Vec3::ONE);
}

#[test]
fn transform_default_equals_identity() {
    let d: Transform = Default::default();
    assert_eq!(d.translation, Transform::IDENTITY.translation);
    assert_eq!(d.rotation, Transform::IDENTITY.rotation);
    assert_eq!(d.scale, Transform::IDENTITY.scale);
}

// ── is_identity ───────────────────────────────────────────────────────────────

#[test]
fn identity_is_identity() {
    assert!(Transform::IDENTITY.is_identity());
}
#[test]
fn default_is_identity() {
    assert!(Transform::default().is_identity());
}

#[test]
fn non_zero_translation_not_identity() {
    let t = Transform::IDENTITY.with_translation(Vec3::new(0.001, 0.0, 0.0));
    assert!(!t.is_identity());
}

#[test]
fn non_unit_scale_not_identity() {
    let t = Transform::IDENTITY.with_scale(Vec3::splat(1.001));
    assert!(!t.is_identity());
}

#[test]
fn non_identity_rotation_not_identity() {
    let t = Transform::IDENTITY.with_rotation(Quat::from_rotation_y(0.01));
    assert!(!t.is_identity());
}

// ── Builder methods ───────────────────────────────────────────────────────────

#[test]
fn with_translation_sets_translation() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    let t = Transform::IDENTITY.with_translation(v);
    assert_eq!(t.translation, v);
}

#[test]
fn with_translation_preserves_rotation_and_scale() {
    let t = Transform::IDENTITY.with_translation(Vec3::X);
    assert_eq!(t.rotation, Quat::IDENTITY);
    assert_eq!(t.scale, Vec3::ONE);
}

#[test]
fn with_rotation_sets_rotation() {
    let q = Quat::from_rotation_y(PI);
    let t = Transform::IDENTITY.with_rotation(q);
    assert!(t.rotation.abs_diff_eq(q, 1e-5));
}

#[test]
fn with_scale_sets_scale() {
    let s = Vec3::new(2.0, 3.0, 4.0);
    let t = Transform::IDENTITY.with_scale(s);
    assert_eq!(t.scale, s);
}

#[test]
fn chain_all_three_setters() {
    let t = Vec3::new(1.0, 0.0, 0.0);
    let r = Quat::from_rotation_z(FRAC_PI_2);
    let s = Vec3::splat(2.0);
    let xf = Transform::IDENTITY
        .with_translation(t)
        .with_rotation(r)
        .with_scale(s);
    assert_eq!(xf.translation, t);
    assert!(xf.rotation.abs_diff_eq(r, 1e-5));
    assert_eq!(xf.scale, s);
}

// ── Matrix round-trip ─────────────────────────────────────────────────────────

#[test]
fn identity_to_matrix_is_identity_matrix() {
    let m = Transform::IDENTITY.to_matrix();
    assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-5));
}

#[test]
fn from_matrix_identity_roundtrip() {
    let t = Transform::from_matrix(Mat4::IDENTITY);
    assert!(t.is_identity());
}

#[test]
fn translation_roundtrip_through_matrix() {
    let orig = Transform::IDENTITY.with_translation(Vec3::new(3.0, -2.0, 5.0));
    let rt = Transform::from_matrix(orig.to_matrix());
    assert!(rt.translation.abs_diff_eq(orig.translation, 1e-4));
}

#[test]
fn scale_roundtrip_through_matrix() {
    let orig = Transform::IDENTITY.with_scale(Vec3::new(2.0, 3.0, 0.5));
    let rt = Transform::from_matrix(orig.to_matrix());
    assert!(rt.scale.abs_diff_eq(orig.scale, 1e-4));
}

#[test]
fn rotation_roundtrip_through_matrix() {
    let q = Quat::from_rotation_x(FRAC_PI_2).normalize();
    let orig = Transform::IDENTITY.with_rotation(q);
    let rt = Transform::from_matrix(orig.to_matrix());
    // Quaternion may negate but represent the same rotation
    let same = rt.rotation.abs_diff_eq(q, 1e-4) || rt.rotation.abs_diff_eq(-q, 1e-4);
    assert!(same);
}

#[test]
fn full_trs_roundtrip() {
    let orig = Transform {
        translation: Vec3::new(1.0, -3.0, 7.0),
        rotation: Quat::from_euler(glam::EulerRot::XYZ, 0.3, 1.2, -0.7).normalize(),
        scale: Vec3::new(2.0, 0.5, 1.5),
    };
    let rt = Transform::from_matrix(orig.to_matrix());
    assert!(rt.translation.abs_diff_eq(orig.translation, 1e-3));
    assert!(rt.scale.abs_diff_eq(orig.scale, 1e-3));
}

// ── Clone & PartialEq ─────────────────────────────────────────────────────────

#[test]
fn transform_clone_equals_original() {
    let t = Transform::IDENTITY.with_translation(Vec3::ONE);
    assert_eq!(t.clone(), t);
}

#[test]
fn transform_partial_eq_same() {
    let a = Transform::IDENTITY.with_translation(Vec3::X);
    let b = Transform::IDENTITY.with_translation(Vec3::X);
    assert_eq!(a, b);
}

#[test]
fn transform_partial_eq_different_translation() {
    let a = Transform::IDENTITY.with_translation(Vec3::X);
    let b = Transform::IDENTITY.with_translation(Vec3::Y);
    assert_ne!(a, b);
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn transform_uniform_scale_2() {
    let t = Transform::IDENTITY.with_scale(Vec3::splat(2.0));
    let m = t.to_matrix();
    // A point at (1,0,0) should end up at (2,0,0)
    let p = m.transform_point3(Vec3::X);
    assert!((p - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-5);
}

#[test]
fn transform_translation_moves_point() {
    let offset = Vec3::new(5.0, 0.0, 0.0);
    let t = Transform::IDENTITY.with_translation(offset);
    let m = t.to_matrix();
    let p = m.transform_point3(Vec3::ZERO);
    assert!(p.abs_diff_eq(offset, 1e-5));
}

#[test]
fn transform_90_degree_rotation() {
    let q = Quat::from_rotation_y(FRAC_PI_2);
    let t = Transform::IDENTITY.with_rotation(q);
    let m = t.to_matrix();
    // Z-axis rotated 90° around Y becomes X-axis
    let z = m.transform_vector3(Vec3::Z);
    assert!(
        z.abs_diff_eq(Vec3::new(-1.0, 0.0, 0.0), 1e-4)
            || z.abs_diff_eq(Vec3::new(1.0, 0.0, 0.0), 1e-4)
    );
}

#[test]
fn zero_scale_determinant_is_zero() {
    let t = Transform::IDENTITY.with_scale(Vec3::ZERO);
    let det = t.to_matrix().determinant();
    assert_eq!(det, 0.0);
}
