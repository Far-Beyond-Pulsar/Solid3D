mod common;
use solid_rs::prelude::*;

// ── Topology defaults ─────────────────────────────────────────────────────────

#[test]
fn topology_default_is_triangle_list() {
    assert_eq!(Topology::default(), Topology::TriangleList);
}

#[test]
fn topology_name_triangle_list() {
    assert_eq!(Topology::TriangleList.name(), "TriangleList");
}
#[test]
fn topology_name_triangle_strip() {
    assert_eq!(Topology::TriangleStrip.name(), "TriangleStrip");
}
#[test]
fn topology_name_line_list() {
    assert_eq!(Topology::LineList.name(), "LineList");
}
#[test]
fn topology_name_line_strip() {
    assert_eq!(Topology::LineStrip.name(), "LineStrip");
}
#[test]
fn topology_name_point_list() {
    assert_eq!(Topology::PointList.name(), "PointList");
}
#[test]
fn topology_name_quad_list() {
    assert_eq!(Topology::QuadList.name(), "QuadList");
}
#[test]
fn topology_name_polygon() {
    assert_eq!(Topology::Polygon.name(), "Polygon");
}

// ── Primitive constructors ────────────────────────────────────────────────────

#[test]
fn primitive_triangles_sets_topology() {
    let p = Primitive::triangles(vec![0, 1, 2], None);
    assert_eq!(p.topology, Topology::TriangleList);
}

#[test]
fn primitive_triangles_stores_indices() {
    let p = Primitive::triangles(vec![0, 1, 2, 3, 4, 5], None);
    assert_eq!(p.indices, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn primitive_triangles_material_none() {
    let p = Primitive::triangles(vec![0, 1, 2], None);
    assert!(p.material_index.is_none());
}

#[test]
fn primitive_triangles_material_some() {
    let p = Primitive::triangles(vec![0, 1, 2], Some(3));
    assert_eq!(p.material_index, Some(3));
}

#[test]
fn primitive_lines_sets_topology() {
    let p = Primitive::lines(vec![0, 1], None);
    assert_eq!(p.topology, Topology::LineList);
}

#[test]
fn primitive_points_sets_topology() {
    let p = Primitive::points(vec![0, 1, 2, 3], None);
    assert_eq!(p.topology, Topology::PointList);
}

// ── element_count ─────────────────────────────────────────────────────────────

#[test]
fn element_count_triangle_list_6_indices() {
    let p = Primitive::triangles(vec![0, 1, 2, 3, 4, 5], None);
    assert_eq!(p.element_count(), 2);
}

#[test]
fn element_count_triangle_list_3_indices() {
    let p = Primitive::triangles(vec![0, 1, 2], None);
    assert_eq!(p.element_count(), 1);
}

#[test]
fn element_count_triangle_list_empty() {
    let p = Primitive::triangles(vec![], None);
    assert_eq!(p.element_count(), 0);
}

#[test]
fn element_count_triangle_strip_4_indices() {
    let p = Primitive {
        topology: Topology::TriangleStrip,
        indices: vec![0, 1, 2, 3],
        material_index: None,
    };
    assert_eq!(p.element_count(), 2); // 4 - 2 = 2
}

#[test]
fn element_count_triangle_strip_empty() {
    let p = Primitive {
        topology: Topology::TriangleStrip,
        indices: vec![],
        material_index: None,
    };
    assert_eq!(p.element_count(), 0); // saturating_sub
}

#[test]
fn element_count_line_list_4_indices() {
    let p = Primitive::lines(vec![0, 1, 2, 3], None);
    assert_eq!(p.element_count(), 2);
}

#[test]
fn element_count_line_strip_4_indices() {
    let p = Primitive {
        topology: Topology::LineStrip,
        indices: vec![0, 1, 2, 3],
        material_index: None,
    };
    assert_eq!(p.element_count(), 3); // 4 - 1
}

#[test]
fn element_count_point_list_5() {
    let p = Primitive::points(vec![0, 1, 2, 3, 4], None);
    assert_eq!(p.element_count(), 5);
}

#[test]
fn element_count_quad_list_8_indices() {
    let p = Primitive {
        topology: Topology::QuadList,
        indices: vec![0, 1, 2, 3, 4, 5, 6, 7],
        material_index: None,
    };
    assert_eq!(p.element_count(), 2);
}

#[test]
fn element_count_polygon_always_1() {
    let p = Primitive {
        topology: Topology::Polygon,
        indices: vec![0, 1, 2, 3, 4, 5],
        material_index: None,
    };
    assert_eq!(p.element_count(), 1);
}

// ── is_empty ──────────────────────────────────────────────────────────────────

#[test]
fn primitive_is_empty_true_when_no_indices() {
    assert!(Primitive::triangles(vec![], None).is_empty());
}

#[test]
fn primitive_is_empty_false_when_has_indices() {
    assert!(!Primitive::triangles(vec![0, 1, 2], None).is_empty());
}

// ── Clone & PartialEq ─────────────────────────────────────────────────────────

#[test]
fn primitive_clone_is_equal() {
    let p = Primitive::triangles(vec![0, 1, 2, 3, 4, 5], Some(1));
    assert_eq!(p.clone(), p);
}

#[test]
fn primitive_partial_eq_same() {
    let a = Primitive::triangles(vec![0, 1, 2], Some(0));
    let b = Primitive::triangles(vec![0, 1, 2], Some(0));
    assert_eq!(a, b);
}

#[test]
fn primitive_partial_eq_different_indices() {
    let a = Primitive::triangles(vec![0, 1, 2], None);
    let b = Primitive::triangles(vec![0, 1, 3], None);
    assert_ne!(a, b);
}

#[test]
fn primitive_partial_eq_different_topology() {
    let a = Primitive {
        topology: Topology::TriangleList,
        indices: vec![0, 1, 2],
        material_index: None,
    };
    let b = Primitive {
        topology: Topology::TriangleStrip,
        indices: vec![0, 1, 2],
        material_index: None,
    };
    assert_ne!(a, b);
}

#[test]
fn primitive_partial_eq_different_material() {
    let a = Primitive::triangles(vec![0, 1, 2], Some(0));
    let b = Primitive::triangles(vec![0, 1, 2], Some(1));
    assert_ne!(a, b);
}

// ── Topology equality / hash ──────────────────────────────────────────────────

#[test]
fn topology_eq() {
    assert_eq!(Topology::TriangleList, Topology::TriangleList);
    assert_ne!(Topology::TriangleList, Topology::LineList);
}

#[test]
fn topology_copy() {
    let t = Topology::LineStrip;
    let t2 = t; // Copy
    assert_eq!(t, t2);
}

// ── Large index buffers ───────────────────────────────────────────────────────

#[test]
fn primitive_large_index_buffer_element_count() {
    let indices: Vec<u32> = (0..30_000).collect();
    let p = Primitive::triangles(indices, None);
    assert_eq!(p.element_count(), 10_000);
}
