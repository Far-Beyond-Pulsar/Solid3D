mod common;
use common::*;
use glam::Vec3;
use solid_rs::prelude::*;

// ── Scene::new / named / default ─────────────────────────────────────────────

#[test]
fn scene_new_name_empty() {
    assert!(Scene::new().name.is_empty());
}
#[test]
fn scene_default_name_empty() {
    assert!(Scene::default().name.is_empty());
}
#[test]
fn scene_named_sets_name() {
    assert_eq!(Scene::named("Demo").name, "Demo");
}
#[test]
fn scene_new_no_roots() {
    assert!(Scene::new().roots.is_empty());
}
#[test]
fn scene_new_no_nodes() {
    assert!(Scene::new().nodes.is_empty());
}
#[test]
fn scene_new_no_meshes() {
    assert!(Scene::new().meshes.is_empty());
}
#[test]
fn scene_new_no_materials() {
    assert!(Scene::new().materials.is_empty());
}
#[test]
fn scene_new_no_textures() {
    assert!(Scene::new().textures.is_empty());
}
#[test]
fn scene_new_no_images() {
    assert!(Scene::new().images.is_empty());
}
#[test]
fn scene_new_no_cameras() {
    assert!(Scene::new().cameras.is_empty());
}
#[test]
fn scene_new_no_lights() {
    assert!(Scene::new().lights.is_empty());
}
#[test]
fn scene_new_no_skins() {
    assert!(Scene::new().skins.is_empty());
}
#[test]
fn scene_new_no_animations() {
    assert!(Scene::new().animations.is_empty());
}
#[test]
fn scene_extensions_empty() {
    assert!(Scene::new().extensions.is_empty());
}

// ── node lookup ──────────────────────────────────────────────────────────────

#[test]
fn scene_node_found() {
    let scene = make_triangle_scene();
    let id = scene.roots[0];
    assert!(scene.node(id).is_some());
}

#[test]
fn scene_node_not_found() {
    let scene = make_triangle_scene();
    assert!(scene.node(NodeId(9999)).is_none());
}

#[test]
fn scene_node_correct_name() {
    let scene = make_triangle_scene();
    let id = scene.roots[0];
    assert_eq!(scene.node(id).unwrap().name, "Root");
}

#[test]
fn scene_node_mut_modifies_name() {
    let mut scene = make_triangle_scene();
    let id = scene.roots[0];
    scene.node_mut(id).unwrap().name = "Renamed".into();
    assert_eq!(scene.node(id).unwrap().name, "Renamed");
}

#[test]
fn scene_node_mut_not_found() {
    let mut scene = Scene::new();
    assert!(scene.node_mut(NodeId(0)).is_none());
}

// ── statistics ───────────────────────────────────────────────────────────────

#[test]
fn scene_total_vertex_count_empty() {
    assert_eq!(Scene::new().total_vertex_count(), 0);
}

#[test]
fn scene_total_vertex_count_triangle_scene() {
    let s = make_triangle_scene();
    assert_eq!(s.total_vertex_count(), 3);
}

#[test]
fn scene_total_vertex_count_two_meshes() {
    let mut b = SceneBuilder::new();
    let mut m1 = Mesh::new("A");
    m1.vertices = vec![Vertex::new(Vec3::ZERO); 5];
    let mut m2 = Mesh::new("B");
    m2.vertices = vec![Vertex::new(Vec3::ONE); 8];
    b.push_mesh(m1);
    b.push_mesh(m2);
    assert_eq!(b.build().total_vertex_count(), 13);
}

#[test]
fn scene_total_index_count_triangle() {
    let s = make_triangle_scene();
    assert_eq!(s.total_index_count(), 3);
}

// ── walk_from ────────────────────────────────────────────────────────────────

#[test]
fn walk_from_visits_root() {
    let scene = make_triangle_scene();
    let root = scene.roots[0];
    let mut visited = vec![];
    scene.walk_from(root, &mut |n| visited.push(n.name.clone()));
    assert!(visited.contains(&"Root".to_owned()));
}

#[test]
fn walk_from_visits_children() {
    let scene = make_deep_hierarchy_scene(3);
    let root = scene.roots[0];
    let mut count = 0;
    scene.walk_from(root, &mut |_| count += 1);
    assert_eq!(count, 4); // root + 3 children
}

#[test]
fn walk_from_nonexistent_root_visits_nothing() {
    let scene = make_triangle_scene();
    let mut count = 0;
    scene.walk_from(NodeId(9999), &mut |_| count += 1);
    assert_eq!(count, 0);
}

// ── walk_all ─────────────────────────────────────────────────────────────────

#[test]
fn walk_all_visits_all_nodes() {
    let scene = make_full_scene();
    let mut count = 0;
    scene.walk_all(&mut |_| count += 1);
    assert_eq!(count, scene.nodes.len());
}

// ── Scene::visit ─────────────────────────────────────────────────────────────

#[test]
fn scene_visit_counts_all_object_types() {
    let scene = make_full_scene();
    let mut vis = CountingVisitor::default();
    scene.visit(&mut vis).unwrap();
    assert_eq!(vis.nodes, scene.nodes.len());
    assert_eq!(vis.meshes, scene.meshes.len());
    assert_eq!(vis.materials, scene.materials.len());
    assert_eq!(vis.textures, scene.textures.len());
    assert_eq!(vis.cameras, scene.cameras.len());
    assert_eq!(vis.lights, scene.lights.len());
    assert_eq!(vis.animations, scene.animations.len());
}

#[test]
fn scene_visit_empty_scene() {
    let mut vis = CountingVisitor::default();
    Scene::new().visit(&mut vis).unwrap();
    assert_eq!(vis.nodes, 0);
    assert_eq!(vis.meshes, 0);
}

#[test]
fn scene_visit_propagates_error() {
    let scene = make_triangle_scene();
    let mut e = ErrorOnMesh;
    assert!(scene.visit(&mut e).is_err());
}

// ── Metadata ─────────────────────────────────────────────────────────────────

#[test]
fn scene_metadata_default_generator_none() {
    assert!(Scene::new().metadata.generator.is_none());
}

#[test]
fn scene_metadata_set_generator() {
    let mut s = Scene::new();
    s.metadata.generator = Some("Blender 4.0".into());
    assert_eq!(s.metadata.generator.as_deref(), Some("Blender 4.0"));
}

#[test]
fn scene_metadata_extra_map() {
    use solid_rs::value::Value;
    let mut s = Scene::new();
    s.metadata.extra.insert("version".into(), Value::Int(2));
    assert_eq!(s.metadata.extra["version"].as_int(), Some(2));
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn scene_clone_preserves_mesh_count() {
    let s = make_quad_scene();
    let c = s.clone();
    assert_eq!(c.meshes.len(), s.meshes.len());
}
