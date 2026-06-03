mod common;
use solid_rs::prelude::*;

// ── new / is_empty / len ──────────────────────────────────────────────────────

#[test]
fn extensions_new_is_empty() {
    assert!(Extensions::new().is_empty());
}
#[test]
fn extensions_default_is_empty() {
    assert!(Extensions::default().is_empty());
}
#[test]
fn extensions_new_len_zero() {
    assert_eq!(Extensions::new().len(), 0);
}

// ── insert & get ─────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
struct TagA(u32);
#[derive(Debug, PartialEq)]
struct TagB(String);
#[derive(Debug)]
struct TagC;

#[test]
fn extensions_insert_and_get_found() {
    let mut e = Extensions::new();
    e.insert(TagA(42));
    assert_eq!(e.get::<TagA>().unwrap().0, 42);
}

#[test]
fn extensions_get_missing_is_none() {
    let e = Extensions::new();
    assert!(e.get::<TagA>().is_none());
}

#[test]
fn extensions_insert_two_types() {
    let mut e = Extensions::new();
    e.insert(TagA(1));
    e.insert(TagB("hello".into()));
    assert_eq!(e.get::<TagA>().unwrap().0, 1);
    assert_eq!(e.get::<TagB>().unwrap().0, "hello");
}

#[test]
fn extensions_insert_replaces_same_type() {
    let mut e = Extensions::new();
    e.insert(TagA(1));
    e.insert(TagA(99));
    assert_eq!(e.get::<TagA>().unwrap().0, 99);
}

#[test]
fn extensions_len_reflects_distinct_types() {
    let mut e = Extensions::new();
    e.insert(TagA(0));
    e.insert(TagB("x".into()));
    assert_eq!(e.len(), 2);
}

#[test]
fn extensions_insert_same_type_twice_len_stays_one() {
    let mut e = Extensions::new();
    e.insert(TagA(0));
    e.insert(TagA(1));
    assert_eq!(e.len(), 1);
}

#[test]
fn extensions_is_not_empty_after_insert() {
    let mut e = Extensions::new();
    e.insert(TagA(0));
    assert!(!e.is_empty());
}

// ── get_mut ───────────────────────────────────────────────────────────────────

#[test]
fn extensions_get_mut_modifies_value() {
    let mut e = Extensions::new();
    e.insert(TagA(10));
    e.get_mut::<TagA>().unwrap().0 = 20;
    assert_eq!(e.get::<TagA>().unwrap().0, 20);
}

#[test]
fn extensions_get_mut_missing_is_none() {
    let mut e = Extensions::new();
    assert!(e.get_mut::<TagA>().is_none());
}

// ── contains ─────────────────────────────────────────────────────────────────

#[test]
fn extensions_contains_after_insert() {
    let mut e = Extensions::new();
    e.insert(TagA(0));
    assert!(e.contains::<TagA>());
}

#[test]
fn extensions_not_contains_before_insert() {
    let e = Extensions::new();
    assert!(!e.contains::<TagA>());
}

#[test]
fn extensions_not_contains_different_type() {
    let mut e = Extensions::new();
    e.insert(TagA(0));
    assert!(!e.contains::<TagB>());
}

// ── remove ────────────────────────────────────────────────────────────────────

#[test]
fn extensions_remove_returns_value() {
    let mut e = Extensions::new();
    e.insert(TagA(55));
    let v = e.remove::<TagA>().unwrap();
    assert_eq!(v.0, 55);
}

#[test]
fn extensions_remove_makes_empty() {
    let mut e = Extensions::new();
    e.insert(TagA(0));
    e.remove::<TagA>();
    assert!(e.is_empty());
}

#[test]
fn extensions_remove_missing_is_none() {
    let mut e = Extensions::new();
    assert!(e.remove::<TagA>().is_none());
}

#[test]
fn extensions_remove_only_correct_type() {
    let mut e = Extensions::new();
    e.insert(TagA(1));
    e.insert(TagB("y".into()));
    e.remove::<TagA>();
    assert!(!e.contains::<TagA>());
    assert!(e.contains::<TagB>());
    assert_eq!(e.len(), 1);
}

// ── clone ─────────────────────────────────────────────────────────────────────

#[test]
fn extensions_clone_is_empty() {
    let mut e = Extensions::new();
    e.insert(TagA(1));
    e.insert(TagC);
    let c = e.clone();
    // Clone intentionally drops data (Box<dyn Any> is not Clone)
    assert!(c.is_empty());
}

// ── Send + Sync ───────────────────────────────────────────────────────────────

#[test]
fn extensions_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Extensions>();
}

#[test]
fn extensions_is_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<Extensions>();
}

// ── With scene objects ─────────────────────────────────────────────────────────

#[test]
fn mesh_extensions_roundtrip() {
    #[derive(Debug)]
    struct LodLevel(u8);
    let mut m = Mesh::new("X");
    m.extensions.insert(LodLevel(3));
    assert_eq!(m.extensions.get::<LodLevel>().unwrap().0, 3);
}

#[test]
fn node_extensions_roundtrip() {
    #[derive(Debug)]
    struct EditorTag {
        label: String,
    }
    let mut n = Node::new(NodeId(0), "X");
    n.extensions.insert(EditorTag {
        label: "selected".into(),
    });
    assert_eq!(n.extensions.get::<EditorTag>().unwrap().label, "selected");
}
