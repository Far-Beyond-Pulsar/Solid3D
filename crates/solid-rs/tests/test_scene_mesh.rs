mod common;
use glam::Vec3;
use solid_rs::prelude::*;

// ── Mesh::new ─────────────────────────────────────────────────────────────────

#[test]
fn mesh_new_sets_name() {
    let m = Mesh::new("Cube");
    assert_eq!(m.name, "Cube");
}

#[test]
fn mesh_new_vertices_empty() {
    assert!(Mesh::new("X").vertices.is_empty());
}
#[test]
fn mesh_new_primitives_empty() {
    assert!(Mesh::new("X").primitives.is_empty());
}
#[test]
fn mesh_new_morph_targets_empty() {
    assert!(Mesh::new("X").morph_targets.is_empty());
}
#[test]
fn mesh_new_bounds_none() {
    assert!(Mesh::new("X").bounds.is_none());
}
#[test]
fn mesh_new_extensions_empty() {
    assert!(Mesh::new("X").extensions.is_empty());
}

// ── is_empty ──────────────────────────────────────────────────────────────────

#[test]
fn mesh_is_empty_initially() {
    assert!(Mesh::new("X").is_empty());
}

#[test]
fn mesh_not_empty_with_vertices() {
    let mut m = Mesh::new("X");
    m.vertices.push(Vertex::new(Vec3::ZERO));
    assert!(!m.is_empty());
}

// ── vertex_count ─────────────────────────────────────────────────────────────

#[test]
fn mesh_vertex_count_zero() {
    assert_eq!(Mesh::new("X").vertex_count(), 0);
}

#[test]
fn mesh_vertex_count_matches_vec_len() {
    let mut m = Mesh::new("X");
    m.vertices = (0..7).map(|i| Vertex::new(Vec3::splat(i as f32))).collect();
    assert_eq!(m.vertex_count(), 7);
}

// ── total_indices ─────────────────────────────────────────────────────────────

#[test]
fn mesh_total_indices_zero_no_primitives() {
    assert_eq!(Mesh::new("X").total_indices(), 0);
}

#[test]
fn mesh_total_indices_single_primitive() {
    let mut m = Mesh::new("X");
    m.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    assert_eq!(m.total_indices(), 3);
}

#[test]
fn mesh_total_indices_two_primitives() {
    let mut m = Mesh::new("X");
    m.primitives = vec![
        Primitive::triangles(vec![0, 1, 2], None),
        Primitive::triangles(vec![3, 4, 5, 6, 7, 8], None),
    ];
    assert_eq!(m.total_indices(), 9);
}

// ── compute_bounds ────────────────────────────────────────────────────────────

#[test]
fn compute_bounds_on_empty_mesh_gives_none() {
    let mut m = Mesh::new("X");
    m.compute_bounds();
    assert!(m.bounds.is_none());
}

#[test]
fn compute_bounds_single_vertex() {
    let p = Vec3::new(3.0, -2.0, 5.0);
    let mut m = Mesh::new("X");
    m.vertices = vec![Vertex::new(p)];
    m.compute_bounds();
    let b = m.bounds.as_ref().unwrap();
    assert_eq!(b.min, p);
    assert_eq!(b.max, p);
}

#[test]
fn compute_bounds_multiple_vertices() {
    let mut m = Mesh::new("X");
    m.vertices = vec![
        Vertex::new(Vec3::new(-1.0, -2.0, -3.0)),
        Vertex::new(Vec3::new(4.0, 5.0, 6.0)),
    ];
    m.compute_bounds();
    let b = m.bounds.as_ref().unwrap();
    assert_eq!(b.min, Vec3::new(-1.0, -2.0, -3.0));
    assert_eq!(b.max, Vec3::new(4.0, 5.0, 6.0));
}

#[test]
fn compute_bounds_overwrites_previous() {
    let mut m = Mesh::new("X");
    m.vertices = vec![Vertex::new(Vec3::ONE)];
    m.compute_bounds();
    m.vertices.push(Vertex::new(Vec3::splat(10.0)));
    m.compute_bounds();
    assert_eq!(m.bounds.as_ref().unwrap().max, Vec3::splat(10.0));
}

// ── MorphTarget ──────────────────────────────────────────────────────────────

#[test]
fn morph_target_default() {
    let mt = MorphTarget::default();
    assert!(mt.name.is_empty());
    assert!(mt.position_deltas.is_empty());
    assert!(mt.normal_deltas.is_empty());
    assert!(mt.tangent_deltas.is_empty());
}

#[test]
fn morph_target_with_name() {
    let mt = MorphTarget {
        name: "smile".into(),
        ..Default::default()
    };
    assert_eq!(mt.name, "smile");
}

#[test]
fn morph_target_position_deltas() {
    let mt = MorphTarget {
        name: "blink".into(),
        position_deltas: vec![Vec3::new(0.0, -0.1, 0.0)],
        ..Default::default()
    };
    assert_eq!(mt.position_deltas.len(), 1);
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn mesh_clone_preserves_name_and_vertex_count() {
    let mut m = Mesh::new("Icosphere");
    m.vertices = vec![Vertex::new(Vec3::ZERO); 12];
    let c = m.clone();
    assert_eq!(c.name, "Icosphere");
    assert_eq!(c.vertex_count(), 12);
}

// ── Extensions ────────────────────────────────────────────────────────────────

#[test]
fn mesh_extension_insert_and_retrieve() {
    #[derive(Debug)]
    struct LODLevel(u8);
    let mut m = Mesh::new("X");
    m.extensions.insert(LODLevel(2));
    assert_eq!(m.extensions.get::<LODLevel>().unwrap().0, 2);
}

// ── Large mesh ────────────────────────────────────────────────────────────────

#[test]
fn large_mesh_vertex_count() {
    let mut m = Mesh::new("Large");
    m.vertices = (0..10_000)
        .map(|i| Vertex::new(Vec3::splat(i as f32)))
        .collect();
    assert_eq!(m.vertex_count(), 10_000);
}

#[test]
fn large_mesh_total_indices() {
    let mut m = Mesh::new("Large");
    let indices: Vec<u32> = (0..30_000).collect();
    m.primitives = vec![Primitive::triangles(indices, None)];
    assert_eq!(m.total_indices(), 30_000);
}
