mod common;
use solid_rs::prelude::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

// ── PerspectiveCamera defaults ────────────────────────────────────────────────

#[test]
fn perspective_default_fov_y() {
    assert!((PerspectiveCamera::default().fov_y - FRAC_PI_4).abs() < 1e-6);
}
#[test]
fn perspective_default_aspect_none() {
    assert!(PerspectiveCamera::default().aspect_ratio.is_none());
}
#[test]
fn perspective_default_z_near() {
    assert!((PerspectiveCamera::default().z_near - 0.01).abs() < 1e-6);
}
#[test]
fn perspective_default_z_far_none() {
    assert!(PerspectiveCamera::default().z_far.is_none());
}

// ── OrthographicCamera defaults ───────────────────────────────────────────────

#[test]
fn ortho_default_x_mag() {
    assert!((OrthographicCamera::default().x_mag - 1.0).abs() < 1e-6);
}
#[test]
fn ortho_default_y_mag() {
    assert!((OrthographicCamera::default().y_mag - 1.0).abs() < 1e-6);
}
#[test]
fn ortho_default_z_near() {
    assert!((OrthographicCamera::default().z_near - 0.01).abs() < 1e-6);
}
#[test]
fn ortho_default_z_far() {
    assert!((OrthographicCamera::default().z_far - 1000.0).abs() < 1e-6);
}

// ── Projection default ────────────────────────────────────────────────────────

#[test]
fn projection_default_is_perspective() {
    assert!(matches!(Projection::default(), Projection::Perspective(_)));
}

// ── Camera::perspective ───────────────────────────────────────────────────────

#[test]
fn camera_perspective_sets_name() {
    let c = Camera::perspective("MainCam");
    assert_eq!(c.name, "MainCam");
}

#[test]
fn camera_perspective_is_perspective() {
    assert!(Camera::perspective("X").is_perspective());
}

#[test]
fn camera_perspective_projection_type() {
    let c = Camera::perspective("X");
    assert!(matches!(c.projection, Projection::Perspective(_)));
}

// ── Camera::orthographic ──────────────────────────────────────────────────────

#[test]
fn camera_orthographic_sets_name() {
    let c = Camera::orthographic("OrthoView");
    assert_eq!(c.name, "OrthoView");
}

#[test]
fn camera_orthographic_not_is_perspective() {
    assert!(!Camera::orthographic("X").is_perspective());
}

#[test]
fn camera_orthographic_projection_type() {
    let c = Camera::orthographic("X");
    assert!(matches!(c.projection, Projection::Orthographic(_)));
}

// ── Extensions ────────────────────────────────────────────────────────────────

#[test]
fn camera_extensions_empty() {
    assert!(Camera::perspective("X").extensions.is_empty());
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn camera_clone_preserves_name() {
    let c = Camera::perspective("Clone");
    assert_eq!(c.clone().name, "Clone");
}

#[test]
fn camera_clone_preserves_projection() {
    let c = Camera::orthographic("O");
    let c2 = c.clone();
    assert!(!c2.is_perspective());
}

// ── Custom perspective ────────────────────────────────────────────────────────

#[test]
fn perspective_custom_fov() {
    let mut c = Camera::perspective("Wide");
    if let Projection::Perspective(ref mut p) = c.projection {
        p.fov_y = FRAC_PI_2;
    }
    if let Projection::Perspective(ref p) = c.projection {
        assert!((p.fov_y - FRAC_PI_2).abs() < 1e-6);
    }
}

#[test]
fn perspective_finite_z_far() {
    let mut cam = Camera::perspective("Finite");
    if let Projection::Perspective(ref mut p) = cam.projection {
        p.z_far = Some(1000.0);
    }
    if let Projection::Perspective(ref p) = cam.projection {
        assert_eq!(p.z_far, Some(1000.0));
    }
}

#[test]
fn perspective_explicit_aspect_ratio() {
    let mut cam = Camera::perspective("HD");
    if let Projection::Perspective(ref mut p) = cam.projection {
        p.aspect_ratio = Some(16.0 / 9.0);
    }
    if let Projection::Perspective(ref p) = cam.projection {
        assert!(p.aspect_ratio.is_some());
    }
}
