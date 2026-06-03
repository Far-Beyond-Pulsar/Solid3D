mod common;
use glam::Vec3;
use solid_rs::prelude::*;

// ── LightBase::new ────────────────────────────────────────────────────────────

#[test]
fn light_base_new_sets_name() {
    let b = LightBase::new("MyLight");
    assert_eq!(b.name, "MyLight");
}

#[test]
fn light_base_new_color_white() {
    assert_eq!(LightBase::new("X").color, Vec3::ONE);
}
#[test]
fn light_base_new_intensity_one() {
    assert_eq!(LightBase::new("X").intensity, 1.0);
}

// ── Light variants ────────────────────────────────────────────────────────────

fn make_dir_light(name: &str) -> Light {
    Light::Directional(DirectionalLight {
        base: LightBase::new(name),
        extensions: Extensions::new(),
    })
}

fn make_point_light(name: &str, range: Option<f32>) -> Light {
    Light::Point(PointLight {
        base: LightBase::new(name),
        range,
        extensions: Extensions::new(),
    })
}

fn make_spot_light(name: &str) -> Light {
    Light::Spot(SpotLight {
        base: LightBase::new(name),
        range: None,
        inner_cone_angle: 0.2,
        outer_cone_angle: 0.5,
        extensions: Extensions::new(),
    })
}

fn make_area_light(name: &str) -> Light {
    Light::Area(AreaLight {
        base: LightBase::new(name),
        width: 2.0,
        height: 1.5,
        extensions: Extensions::new(),
    })
}

// ── name() ───────────────────────────────────────────────────────────────────

#[test]
fn directional_light_name() {
    assert_eq!(make_dir_light("Sun").name(), "Sun");
}
#[test]
fn point_light_name() {
    assert_eq!(make_point_light("Lamp", None).name(), "Lamp");
}
#[test]
fn spot_light_name() {
    assert_eq!(make_spot_light("Spot").name(), "Spot");
}
#[test]
fn area_light_name() {
    assert_eq!(make_area_light("Area").name(), "Area");
}

// ── color() ──────────────────────────────────────────────────────────────────

#[test]
fn directional_light_color_white() {
    assert_eq!(make_dir_light("X").color(), Vec3::ONE);
}

#[test]
fn point_light_custom_color() {
    let mut l = make_point_light("X", None);
    l.base_mut().color = Vec3::new(1.0, 0.5, 0.0);
    assert_eq!(l.color(), Vec3::new(1.0, 0.5, 0.0));
}

// ── intensity() ──────────────────────────────────────────────────────────────

#[test]
fn directional_light_intensity_default() {
    assert_eq!(make_dir_light("X").intensity(), 1.0);
}

#[test]
fn point_light_custom_intensity() {
    let mut l = make_point_light("X", None);
    l.base_mut().intensity = 5.0;
    assert_eq!(l.intensity(), 5.0);
}

// ── base_mut ─────────────────────────────────────────────────────────────────

#[test]
fn base_mut_directional() {
    let mut l = make_dir_light("X");
    l.base_mut().name = "Updated".into();
    assert_eq!(l.name(), "Updated");
}

#[test]
fn base_mut_point() {
    let mut l = make_point_light("X", None);
    l.base_mut().intensity = 100.0;
    assert_eq!(l.intensity(), 100.0);
}

#[test]
fn base_mut_spot() {
    let mut l = make_spot_light("X");
    l.base_mut().color = Vec3::new(0.0, 0.0, 1.0);
    assert_eq!(l.color(), Vec3::new(0.0, 0.0, 1.0));
}

#[test]
fn base_mut_area() {
    let mut l = make_area_light("X");
    l.base_mut().name = "BigArea".into();
    assert_eq!(l.name(), "BigArea");
}

// ── PointLight range ──────────────────────────────────────────────────────────

#[test]
fn point_light_no_range() {
    if let Light::Point(ref p) = make_point_light("X", None) {
        assert!(p.range.is_none());
    }
}

#[test]
fn point_light_with_range() {
    if let Light::Point(ref p) = make_point_light("X", Some(50.0)) {
        assert_eq!(p.range, Some(50.0));
    }
}

// ── SpotLight angles ──────────────────────────────────────────────────────────

#[test]
fn spot_light_cone_angles() {
    if let Light::Spot(ref s) = make_spot_light("X") {
        assert_eq!(s.inner_cone_angle, 0.2);
        assert_eq!(s.outer_cone_angle, 0.5);
        assert!(s.inner_cone_angle < s.outer_cone_angle);
    }
}

// ── AreaLight dimensions ──────────────────────────────────────────────────────

#[test]
fn area_light_dimensions() {
    if let Light::Area(ref a) = make_area_light("X") {
        assert_eq!(a.width, 2.0);
        assert_eq!(a.height, 1.5);
    }
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn directional_clone() {
    let l = make_dir_light("SunClone");
    let c = l.clone();
    assert_eq!(c.name(), "SunClone");
}

#[test]
fn point_clone() {
    let l = make_point_light("PointClone", Some(25.0));
    let c = l.clone();
    assert_eq!(c.name(), "PointClone");
    if let Light::Point(ref p) = c {
        assert_eq!(p.range, Some(25.0));
    }
}
