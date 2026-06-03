mod common;
use glam::Vec3;
use solid_rs::prelude::*;

// ── NodeId ────────────────────────────────────────────────────────────────────

#[test]
fn node_id_default_is_zero() {
    assert_eq!(NodeId::default(), NodeId(0));
}
#[test]
fn node_id_equality() {
    assert_eq!(NodeId(5), NodeId(5));
}
#[test]
fn node_id_inequality() {
    assert_ne!(NodeId(1), NodeId(2));
}
#[test]
fn node_id_ordering() {
    assert!(NodeId(1) < NodeId(2));
}
#[test]
fn node_id_copy() {
    let a = NodeId(3);
    let b = a;
    assert_eq!(a, b);
}
#[test]
fn node_id_display() {
    assert_eq!(format!("{}", NodeId(7)), "Node(7)");
}

// ── Node::new ────────────────────────────────────────────────────────────────

#[test]
fn node_new_sets_id_and_name() {
    let n = Node::new(NodeId(42), "TestNode");
    assert_eq!(n.id, NodeId(42));
    assert_eq!(n.name, "TestNode");
}

#[test]
fn node_new_transform_is_identity() {
    let n = Node::new(NodeId(0), "X");
    assert!(n.transform.is_identity());
}

#[test]
fn node_new_no_children() {
    let n = Node::new(NodeId(0), "X");
    assert!(n.children.is_empty());
}

#[test]
fn node_new_no_mesh() {
    assert!(Node::new(NodeId(0), "X").mesh.is_none());
}
#[test]
fn node_new_no_camera() {
    assert!(Node::new(NodeId(0), "X").camera.is_none());
}
#[test]
fn node_new_no_light() {
    assert!(Node::new(NodeId(0), "X").light.is_none());
}
#[test]
fn node_new_no_skin() {
    assert!(Node::new(NodeId(0), "X").skin.is_none());
}

#[test]
fn node_new_extensions_empty() {
    assert!(Node::new(NodeId(0), "X").extensions.is_empty());
}

// ── is_leaf ───────────────────────────────────────────────────────────────────

#[test]
fn node_is_leaf_when_no_children() {
    assert!(Node::new(NodeId(0), "Leaf").is_leaf());
}

#[test]
fn node_is_not_leaf_with_children() {
    let mut n = Node::new(NodeId(0), "Parent");
    n.children.push(NodeId(1));
    assert!(!n.is_leaf());
}

// ── has_attachment ────────────────────────────────────────────────────────────

#[test]
fn node_has_no_attachment_by_default() {
    assert!(!Node::new(NodeId(0), "X").has_attachment());
}

#[test]
fn node_has_attachment_with_mesh() {
    let mut n = Node::new(NodeId(0), "X");
    n.mesh = Some(0);
    assert!(n.has_attachment());
}

#[test]
fn node_has_attachment_with_camera() {
    let mut n = Node::new(NodeId(0), "X");
    n.camera = Some(0);
    assert!(n.has_attachment());
}

#[test]
fn node_has_attachment_with_light() {
    let mut n = Node::new(NodeId(0), "X");
    n.light = Some(0);
    assert!(n.has_attachment());
}

#[test]
fn node_has_attachment_with_skin() {
    let mut n = Node::new(NodeId(0), "X");
    n.skin = Some(0);
    assert!(n.has_attachment());
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn node_clone_preserves_id_and_name() {
    let n = Node::new(NodeId(99), "CloneMe");
    let c = n.clone();
    assert_eq!(c.id, n.id);
    assert_eq!(c.name, n.name);
}

#[test]
fn node_clone_preserves_children() {
    let mut n = Node::new(NodeId(0), "Parent");
    n.children = vec![NodeId(1), NodeId(2), NodeId(3)];
    let c = n.clone();
    assert_eq!(c.children, vec![NodeId(1), NodeId(2), NodeId(3)]);
}

#[test]
fn node_name_from_string() {
    let name = "DynamicName".to_owned();
    let n = Node::new(NodeId(0), name);
    assert_eq!(n.name, "DynamicName");
}

// ── Extensions ────────────────────────────────────────────────────────────────

#[test]
fn node_extensions_insert_and_get() {
    #[derive(Debug)]
    struct Tag(u32);
    let mut n = Node::new(NodeId(0), "X");
    n.extensions.insert(Tag(42));
    assert_eq!(n.extensions.get::<Tag>().unwrap().0, 42);
}
