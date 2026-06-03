mod common;
use common::*;
use solid_rs::prelude::*;
use std::io::Cursor;

// ── new / default ─────────────────────────────────────────────────────────────

#[test]
fn registry_new_no_loaders() {
    let r = Registry::new();
    assert_eq!(r.loader_infos().count(), 0);
}

#[test]
fn registry_new_no_savers() {
    let r = Registry::new();
    assert_eq!(r.saver_infos().count(), 0);
}

#[test]
fn registry_default_same_as_new() {
    assert_eq!(Registry::default().loader_infos().count(), 0);
}

// ── register ─────────────────────────────────────────────────────────────────

#[test]
fn registry_register_loader_adds_one() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert_eq!(r.loader_infos().count(), 1);
}

#[test]
fn registry_register_saver_adds_one() {
    let mut r = Registry::new();
    r.register_saver(MockSaver);
    assert_eq!(r.saver_infos().count(), 1);
}

#[test]
fn registry_register_chaining() {
    let mut r = Registry::new();
    r.register_loader(MockLoader)
        .register_saver(MockSaver)
        .register_loader(XyzLoader)
        .register_saver(XyzSaver);
    assert_eq!(r.loader_infos().count(), 2);
    assert_eq!(r.saver_infos().count(), 2);
}

// ── loader_by_id ──────────────────────────────────────────────────────────────

#[test]
fn loader_by_id_found() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_by_id("mock").is_some());
}

#[test]
fn loader_by_id_not_found() {
    let r = Registry::new();
    assert!(r.loader_by_id("obj").is_none());
}

#[test]
fn loader_by_id_case_insensitive() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_by_id("MOCK").is_some());
    assert!(r.loader_by_id("Mock").is_some());
}

// ── loader_for_extension ──────────────────────────────────────────────────────

#[test]
fn loader_for_extension_found() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_for_extension("mock").is_some());
}

#[test]
fn loader_for_extension_not_found() {
    let r = Registry::new();
    assert!(r.loader_for_extension("fbx").is_none());
}

#[test]
fn loader_for_extension_case_insensitive() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_for_extension("MOCK").is_some());
}

// ── saver_by_id ───────────────────────────────────────────────────────────────

#[test]
fn saver_by_id_found() {
    let mut r = Registry::new();
    r.register_saver(MockSaver);
    assert!(r.saver_by_id("mock").is_some());
}

#[test]
fn saver_by_id_not_found() {
    let r = Registry::new();
    assert!(r.saver_by_id("gltf").is_none());
}

// ── can_load / can_save ───────────────────────────────────────────────────────

#[test]
fn can_load_extension_true() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.can_load_extension("mock"));
}

#[test]
fn can_load_extension_false() {
    let r = Registry::new();
    assert!(!r.can_load_extension("mock"));
}

#[test]
fn can_save_extension_true() {
    let mut r = Registry::new();
    r.register_saver(MockSaver);
    assert!(r.can_save_extension("mock"));
}

#[test]
fn can_save_extension_false() {
    let r = Registry::new();
    assert!(!r.can_save_extension("mock"));
}

// ── load_file errors ──────────────────────────────────────────────────────────

#[test]
fn load_file_no_extension_returns_error() {
    let r = Registry::new();
    let result = r.load_file("model_no_ext");
    assert!(result.is_err());
}

#[test]
fn load_file_unregistered_extension_returns_error() {
    let r = Registry::new();
    let result = r.load_file("model.fbx");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SolidError::UnsupportedFormat(_)
    ));
}

// ── save_file errors ──────────────────────────────────────────────────────────

#[test]
fn save_file_unregistered_extension_returns_error() {
    let r = Registry::new();
    let s = Scene::new();
    let result = r.save_file(&s, "out.fbx");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SolidError::UnsupportedFormat(_)
    ));
}

#[test]
fn save_file_no_extension_returns_error() {
    let r = Registry::new();
    let result = r.save_file(&Scene::new(), "no_extension");
    assert!(result.is_err());
}

// ── load_from ─────────────────────────────────────────────────────────────────

#[test]
fn load_from_known_format_succeeds() {
    let mut r = Registry::new();
    r.register_loader(XyzLoader);
    let data = b"0 0 0\n1 0 0\n0 1 0\n";
    let scene = r
        .load_from(Cursor::new(data), "xyz", &LoadOptions::default())
        .unwrap();
    assert_eq!(scene.meshes[0].vertex_count(), 3);
}

#[test]
fn load_from_unknown_format_returns_error() {
    let r = Registry::new();
    let e = r.load_from(Cursor::new(b""), "unknown", &LoadOptions::default());
    assert!(matches!(e.unwrap_err(), SolidError::UnsupportedFormat(_)));
}

// ── loader_infos / saver_infos ────────────────────────────────────────────────

#[test]
fn loader_infos_lists_all_registered() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    r.register_loader(XyzLoader);
    let ids: Vec<&str> = r.loader_infos().map(|i| i.id).collect();
    assert!(ids.contains(&"mock"));
    assert!(ids.contains(&"xyz"));
}

#[test]
fn saver_infos_empty_when_none() {
    let r = Registry::new();
    assert_eq!(r.saver_infos().count(), 0);
}

// ── loader_for_mime ───────────────────────────────────────────────────────────

#[test]
fn loader_for_mime_found() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    assert!(r.loader_for_mime("model/x-mock").is_some());
}

#[test]
fn loader_for_mime_not_found() {
    let r = Registry::new();
    assert!(r.loader_for_mime("model/obj").is_none());
}

// ── format info on retrieved loader ──────────────────────────────────────────

#[test]
fn loader_format_info_correct_after_lookup() {
    let mut r = Registry::new();
    r.register_loader(MockLoader);
    let info = r.loader_by_id("mock").unwrap().format_info();
    assert_eq!(info.name, "Mock Format");
    assert!(info.can_load);
}

// ── multiple formats ──────────────────────────────────────────────────────────

#[test]
fn registry_routes_xyz_correctly() {
    let mut r = Registry::new();
    r.register_loader(XyzLoader);
    r.register_loader(MockLoader);
    let l = r.loader_for_extension("xyz").unwrap();
    assert_eq!(l.format_info().id, "xyz");
}
