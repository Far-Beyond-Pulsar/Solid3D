mod common;
use glam::Vec3;
use solid_rs::prelude::*;

// ── Aabb::new ─────────────────────────────────────────────────────────────────

#[test]
fn aabb_new_stores_min_max() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert_eq!(a.min, Vec3::ZERO);
    assert_eq!(a.max, Vec3::ONE);
}

#[test]
fn aabb_new_negative_coords() {
    let a = Aabb::new(Vec3::new(-3.0, -2.0, -1.0), Vec3::new(3.0, 2.0, 1.0));
    assert_eq!(a.min, Vec3::new(-3.0, -2.0, -1.0));
}

// ── from_points ───────────────────────────────────────────────────────────────

#[test]
fn aabb_from_points_empty_returns_none() {
    assert!(Aabb::from_points(std::iter::empty()).is_none());
}

#[test]
fn aabb_from_points_single_point() {
    let p = Vec3::new(2.0, 3.0, 4.0);
    let a = Aabb::from_points(std::iter::once(p)).unwrap();
    assert_eq!(a.min, p);
    assert_eq!(a.max, p);
}

#[test]
fn aabb_from_points_two_points() {
    let pts = vec![Vec3::new(1.0, 1.0, 1.0), Vec3::new(-1.0, -1.0, -1.0)];
    let a = Aabb::from_points(pts.into_iter()).unwrap();
    assert_eq!(a.min, Vec3::splat(-1.0));
    assert_eq!(a.max, Vec3::splat(1.0));
}

#[test]
fn aabb_from_points_all_same() {
    let pts: Vec<Vec3> = (0..5).map(|_| Vec3::ONE).collect();
    let a = Aabb::from_points(pts.into_iter()).unwrap();
    assert_eq!(a.min, Vec3::ONE);
    assert_eq!(a.max, Vec3::ONE);
}

#[test]
fn aabb_from_points_mixed() {
    let pts = vec![
        Vec3::new(3.0, -1.0, 0.0),
        Vec3::new(-2.0, 5.0, 1.0),
        Vec3::new(0.0, 0.0, -3.0),
    ];
    let a = Aabb::from_points(pts.into_iter()).unwrap();
    assert_eq!(a.min, Vec3::new(-2.0, -1.0, -3.0));
    assert_eq!(a.max, Vec3::new(3.0, 5.0, 1.0));
}

// ── center ────────────────────────────────────────────────────────────────────

#[test]
fn aabb_center_symmetric_box() {
    let a = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
    assert_eq!(a.center(), Vec3::ZERO);
}

#[test]
fn aabb_center_offset_box() {
    let a = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 4.0, 6.0));
    assert_eq!(a.center(), Vec3::new(1.0, 2.0, 3.0));
}

// ── size & half_extents ───────────────────────────────────────────────────────

#[test]
fn aabb_size_unit_cube() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert_eq!(a.size(), Vec3::ONE);
}

#[test]
fn aabb_size_rectangular() {
    let a = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 4.0, 8.0));
    assert_eq!(a.size(), Vec3::new(2.0, 4.0, 8.0));
}

#[test]
fn aabb_half_extents_unit_cube() {
    let a = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
    assert_eq!(a.half_extents(), Vec3::ONE);
}

#[test]
fn aabb_half_extents_is_half_of_size() {
    let a = Aabb::new(Vec3::ZERO, Vec3::new(4.0, 6.0, 8.0));
    assert_eq!(a.half_extents(), a.size() * 0.5);
}

// ── contains ─────────────────────────────────────────────────────────────────

#[test]
fn aabb_contains_interior_point() {
    let a = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
    assert!(a.contains(Vec3::ZERO));
}

#[test]
fn aabb_contains_on_min_boundary() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert!(a.contains(Vec3::ZERO));
}

#[test]
fn aabb_contains_on_max_boundary() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert!(a.contains(Vec3::ONE));
}

#[test]
fn aabb_does_not_contain_outside_x() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert!(!a.contains(Vec3::new(1.001, 0.5, 0.5)));
}

#[test]
fn aabb_does_not_contain_outside_negative() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert!(!a.contains(Vec3::new(-0.001, 0.5, 0.5)));
}

// ── union ─────────────────────────────────────────────────────────────────────

#[test]
fn aabb_union_of_two_non_overlapping() {
    let a = Aabb::new(Vec3::new(-3.0, -3.0, -3.0), Vec3::new(-1.0, -1.0, -1.0));
    let b = Aabb::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 3.0));
    let u = a.union(&b);
    assert_eq!(u.min, Vec3::splat(-3.0));
    assert_eq!(u.max, Vec3::splat(3.0));
}

#[test]
fn aabb_union_of_identical_is_same() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    let u = a.union(&a);
    assert_eq!(u, a);
}

#[test]
fn aabb_union_subsumes_inner() {
    let outer = Aabb::new(Vec3::splat(-10.0), Vec3::splat(10.0));
    let inner = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
    let u = outer.union(&inner);
    assert_eq!(u.min, outer.min);
    assert_eq!(u.max, outer.max);
}

// ── intersects ───────────────────────────────────────────────────────────────

#[test]
fn aabb_intersects_overlapping_boxes() {
    let a = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
    let b = Aabb::new(Vec3::ONE, Vec3::new(3.0, 3.0, 3.0));
    assert!(a.intersects(&b));
}

#[test]
fn aabb_intersects_touching_face() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    let b = Aabb::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0));
    assert!(a.intersects(&b));
}

#[test]
fn aabb_not_intersects_separated() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    let b = Aabb::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));
    assert!(!a.intersects(&b));
}

// ── surface_area & volume ────────────────────────────────────────────────────

#[test]
fn aabb_surface_area_unit_cube() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert!((a.surface_area() - 6.0).abs() < 1e-5);
}

#[test]
fn aabb_volume_unit_cube() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert!((a.volume() - 1.0).abs() < 1e-5);
}

#[test]
fn aabb_volume_2x3x4_box() {
    let a = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 3.0, 4.0));
    assert!((a.volume() - 24.0).abs() < 1e-5);
}

#[test]
fn aabb_surface_area_2x2x2_cube() {
    let a = Aabb::new(Vec3::ZERO, Vec3::splat(2.0));
    assert!((a.surface_area() - 24.0).abs() < 1e-5);
}

// ── Clone & PartialEq ─────────────────────────────────────────────────────────

#[test]
fn aabb_clone_equals_original() {
    let a = Aabb::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0));
    assert_eq!(a.clone(), a);
}

#[test]
fn aabb_partial_eq_same() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    let b = Aabb::new(Vec3::ZERO, Vec3::ONE);
    assert_eq!(a, b);
}

#[test]
fn aabb_partial_eq_different() {
    let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
    let b = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
    assert_ne!(a, b);
}

// ── Zero-size AABB ────────────────────────────────────────────────────────────

#[test]
fn aabb_zero_size_volume_is_zero() {
    let a = Aabb::new(Vec3::ONE, Vec3::ONE);
    assert_eq!(a.volume(), 0.0);
}

#[test]
fn aabb_zero_size_contains_its_point() {
    let p = Vec3::new(5.0, 5.0, 5.0);
    let a = Aabb::new(p, p);
    assert!(a.contains(p));
}

// ── Large AABB ────────────────────────────────────────────────────────────────

#[test]
fn aabb_from_large_point_cloud() {
    let pts: Vec<Vec3> = (0..1000)
        .map(|i| Vec3::new(i as f32, -(i as f32), (i * 2) as f32))
        .collect();
    let a = Aabb::from_points(pts.into_iter()).unwrap();
    assert_eq!(a.min.x, 0.0);
    assert_eq!(a.max.x, 999.0);
}
