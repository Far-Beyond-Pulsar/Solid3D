mod common;
use common::*;
use glam::{Quat, Vec3};
use solid_rs::prelude::*;

// ── new / named ───────────────────────────────────────────────────────────────

#[test]
fn builder_new_creates_empty_scene() {
    let scene = SceneBuilder::new().build();
    assert!(scene.nodes.is_empty());
    assert!(scene.roots.is_empty());
}

#[test]
fn builder_named_sets_scene_name() {
    let scene = SceneBuilder::named("Test").build();
    assert_eq!(scene.name, "Test");
}

// ── unique IDs ────────────────────────────────────────────────────────────────

#[test]
fn builder_alloc_unique_ids() {
    let mut b = SceneBuilder::new();
    let id_a = b.add_root_node("A");
    let id_b = b.add_root_node("B");
    assert_ne!(id_a, id_b);
}

#[test]
fn builder_ids_sequential() {
    let mut b = SceneBuilder::new();
    let ids: Vec<_> = (0..5).map(|i| b.add_root_node(format!("N{i}"))).collect();
    // All distinct
    let set: std::collections::HashSet<_> = ids.iter().copied().collect();
    assert_eq!(set.len(), 5);
}

// ── add_root_node ─────────────────────────────────────────────────────────────

#[test]
fn builder_add_root_node_appears_in_roots() {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("R");
    let s = b.build();
    assert!(s.roots.contains(&r));
}

#[test]
fn builder_root_node_appears_in_nodes() {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("Root");
    let s = b.build();
    assert!(s.node(r).is_some());
}

#[test]
fn builder_multiple_roots() {
    let mut b = SceneBuilder::new();
    let _r1 = b.add_root_node("R1");
    let _r2 = b.add_root_node("R2");
    let _r3 = b.add_root_node("R3");
    let s = b.build();
    assert_eq!(s.roots.len(), 3);
}

// ── add_child_node ────────────────────────────────────────────────────────────

#[test]
fn builder_child_has_parent() {
    let mut b = SceneBuilder::new();
    let parent = b.add_root_node("Parent");
    let child = b.add_child_node(parent, "Child");
    let s = b.build();
    assert_eq!(s.node(child).unwrap().parent, Some(parent));
}

#[test]
fn builder_child_appears_in_parent_children() {
    let mut b = SceneBuilder::new();
    let parent = b.add_root_node("Parent");
    let child = b.add_child_node(parent, "Child");
    let s = b.build();
    assert!(s.node(parent).unwrap().children.contains(&child));
}

#[test]
fn builder_child_not_in_roots() {
    let mut b = SceneBuilder::new();
    let parent = b.add_root_node("P");
    let child = b.add_child_node(parent, "C");
    let s = b.build();
    assert!(!s.roots.contains(&child));
}

#[test]
fn builder_deep_hierarchy() {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("R");
    let mut current = r;
    for i in 0..10 {
        current = b.add_child_node(current, format!("N{i}"));
    }
    let s = b.build();
    assert_eq!(s.nodes.len(), 11); // root + 10 children
}

// ── set_transform ─────────────────────────────────────────────────────────────

#[test]
fn builder_set_transform() {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("R");
    let t = Transform::default().with_translation(Vec3::new(1.0, 2.0, 3.0));
    b.set_transform(r, t);
    let s = b.build();
    assert_eq!(
        s.node(r).unwrap().transform.translation,
        Vec3::new(1.0, 2.0, 3.0)
    );
}

// ── push_* returns correct indices ────────────────────────────────────────────

#[test]
fn builder_push_mesh_index_0() {
    let mut b = SceneBuilder::new();
    let idx = b.push_mesh(Mesh::new("M"));
    assert_eq!(idx, 0);
}

#[test]
fn builder_push_mesh_increments() {
    let mut b = SceneBuilder::new();
    let idx0 = b.push_mesh(Mesh::new("M0"));
    let idx1 = b.push_mesh(Mesh::new("M1"));
    assert_eq!(idx0, 0);
    assert_eq!(idx1, 1);
}

#[test]
fn builder_push_material_index() {
    let mut b = SceneBuilder::new();
    let idx = b.push_material(Material::default());
    assert_eq!(idx, 0);
}

#[test]
fn builder_push_texture_index() {
    let mut b = SceneBuilder::new();
    let idx = b.push_texture(Texture::new("Tex", 0));
    assert_eq!(idx, 0);
}

#[test]
fn builder_push_image_index() {
    let mut b = SceneBuilder::new();
    let idx = b.push_image(Image::from_uri("Img", "test.png"));
    assert_eq!(idx, 0);
}

#[test]
fn builder_push_camera_index() {
    let mut b = SceneBuilder::new();
    let idx = b.push_camera(Camera::perspective("Cam"));
    assert_eq!(idx, 0);
}

#[test]
fn builder_push_light_index() {
    let mut b = SceneBuilder::new();
    let l = Light::Directional(DirectionalLight {
        base: LightBase::new("Sun"),
        extensions: Extensions::new(),
    });
    let idx = b.push_light(l);
    assert_eq!(idx, 0);
}

#[test]
fn builder_push_skin_index() {
    let mut b = SceneBuilder::new();
    let idx = b.push_skin(Skin::new("Skin0"));
    assert_eq!(idx, 0);
}

#[test]
fn builder_push_animation_index() {
    let mut b = SceneBuilder::new();
    let idx = b.push_animation(Animation::new("Anim0"));
    assert_eq!(idx, 0);
}

// ── attach_* ─────────────────────────────────────────────────────────────────

#[test]
fn builder_attach_mesh_to_node() {
    let mut b = SceneBuilder::new();
    let node_id = b.add_root_node("N");
    let mesh_idx = b.push_mesh(Mesh::new("M"));
    b.attach_mesh(node_id, mesh_idx);
    let s = b.build();
    assert_eq!(s.node(node_id).unwrap().mesh, Some(mesh_idx));
}

#[test]
fn builder_attach_camera_to_node() {
    let mut b = SceneBuilder::new();
    let node_id = b.add_root_node("N");
    let cam_idx = b.push_camera(Camera::perspective("C"));
    b.attach_camera(node_id, cam_idx);
    let s = b.build();
    assert_eq!(s.node(node_id).unwrap().camera, Some(cam_idx));
}

#[test]
fn builder_attach_light_to_node() {
    let mut b = SceneBuilder::new();
    let node_id = b.add_root_node("N");
    let light_idx = b.push_light(Light::Directional(DirectionalLight {
        base: LightBase::new("Sun"),
        extensions: Extensions::new(),
    }));
    b.attach_light(node_id, light_idx);
    let s = b.build();
    assert_eq!(s.node(node_id).unwrap().light, Some(light_idx));
}

#[test]
fn builder_attach_skin_to_node() {
    let mut b = SceneBuilder::new();
    let node_id = b.add_root_node("N");
    let skin_idx = b.push_skin(Skin::new("S"));
    b.attach_skin(node_id, skin_idx);
    let s = b.build();
    assert_eq!(s.node(node_id).unwrap().skin, Some(skin_idx));
}

// ── build produces correctly indexed scenes ───────────────────────────────────

#[test]
fn builder_build_has_all_pushed_meshes() {
    let mut b = SceneBuilder::new();
    b.push_mesh(Mesh::new("A"));
    b.push_mesh(Mesh::new("B"));
    b.push_mesh(Mesh::new("C"));
    let s = b.build();
    assert_eq!(s.meshes.len(), 3);
}

#[test]
fn builder_build_preserves_mesh_names() {
    let mut b = SceneBuilder::new();
    b.push_mesh(Mesh::new("Alpha"));
    b.push_mesh(Mesh::new("Beta"));
    let s = b.build();
    assert_eq!(s.meshes[0].name, "Alpha");
    assert_eq!(s.meshes[1].name, "Beta");
}

// ── complex multi-root scene ──────────────────────────────────────────────────

#[test]
fn builder_complex_scene() {
    let mut b = SceneBuilder::named("Complex");
    let root1 = b.add_root_node("R1");
    let root2 = b.add_root_node("R2");
    let child1 = b.add_child_node(root1, "C1");
    let _child2 = b.add_child_node(root1, "C2");
    let _grandchild = b.add_child_node(child1, "G1");

    let m_idx = b.push_mesh(make_triangle_mesh());
    b.attach_mesh(root1, m_idx);

    let s = b.build();
    assert_eq!(s.roots.len(), 2);
    assert_eq!(s.nodes.len(), 5);
    assert_eq!(s.meshes.len(), 1);
    assert!(s.node(root2).unwrap().children.is_empty());
}
