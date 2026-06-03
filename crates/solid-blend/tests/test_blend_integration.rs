use solid_blend::{BlendLoader, BlendSaver};
use solid_rs::prelude::*;
use std::io::Cursor;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn loader_reports_missing_blender_binary() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    let previous = std::env::var_os("BLENDER_BIN");
    std::env::set_var("BLENDER_BIN", "definitely-not-a-real-blender-bin");

    let blend_header_only = b"BLENDER-v300";
    let err = BlendLoader
        .load(
            &mut Cursor::new(&blend_header_only[..]),
            &LoadOptions::default(),
        )
        .expect_err("expected missing blender error");
    assert!(matches!(err, SolidError::UnsupportedFeature(_)));

    if let Some(value) = previous {
        std::env::set_var("BLENDER_BIN", value);
    } else {
        std::env::remove_var("BLENDER_BIN");
    }
}

#[test]
fn saver_reports_missing_blender_binary() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    let previous = std::env::var_os("BLENDER_BIN");
    std::env::set_var("BLENDER_BIN", "definitely-not-a-real-blender-bin");

    let scene = Scene::new();
    let mut out = Vec::new();
    let err = BlendSaver
        .save(&scene, &mut out, &SaveOptions::default())
        .expect_err("expected missing blender error");
    assert!(matches!(err, SolidError::UnsupportedFeature(_)));

    if let Some(value) = previous {
        std::env::set_var("BLENDER_BIN", value);
    } else {
        std::env::remove_var("BLENDER_BIN");
    }
}
