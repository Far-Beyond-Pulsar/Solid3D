//! Integration tests for MdlSaver.

mod common;

use common::*;
use solid_rs::prelude::*;
use solid_mdl::MdlSaver;

#[test]
fn saver_header_magic_correct() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let mut buf = Vec::new();
    MdlSaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .unwrap();
    // Check magic at offset 0
    let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    assert_eq!(magic, 1330660425, "magic should be 'IDPO'");
}

#[test]
fn saver_header_version_correct() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let mut buf = Vec::new();
    MdlSaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .unwrap();
    let version = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    assert_eq!(version, 6, "version should be 6");
}

#[test]
fn saver_num_tris_correct() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let mut buf = Vec::new();
    MdlSaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .unwrap();
    let num_tris = i32::from_le_bytes([buf[64], buf[65], buf[66], buf[67]]);
    assert_eq!(num_tris, 1, "triangle count should be 1");
}

#[test]
fn saver_num_verts_correct() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let mut buf = Vec::new();
    MdlSaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .unwrap();
    let num_verts = i32::from_le_bytes([buf[60], buf[61], buf[62], buf[63]]);
    assert_eq!(num_verts, 3, "vertex count should be 3");
}

#[test]
fn saver_num_frames_correct() {
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    );
    let mut buf = Vec::new();
    MdlSaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .unwrap();
    let num_frames = i32::from_le_bytes([buf[68], buf[69], buf[70], buf[71]]);
    assert_eq!(num_frames, 1, "frame count should be 1");
}

#[test]
fn saver_empty_scene_error() {
    let scene = Scene::new();
    let mut buf = Vec::new();
    let result = MdlSaver.save(&scene, &mut buf, &SaveOptions::default());
    assert!(result.is_err(), "empty scene should produce an error");
}
