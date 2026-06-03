//! Shared test helpers for solid-usd integration tests.
#![allow(dead_code)]

use solid_rs::prelude::Loader;
use solid_rs::prelude::Saver;

use glam::{Vec3, Vec4};
use solid_rs::{
    builder::SceneBuilder,
    geometry::{Primitive, Vertex},
    prelude::{LoadOptions, SaveOptions},
    scene::{Material, Mesh, Scene},
};
use solid_usd::{UsdLoader, UsdSaver};
use std::io::Cursor;

// ── Scene factories ───────────────────────────────────────────────────────────

/// A minimal triangle scene.
pub fn triangle_scene() -> Scene {
    let mut b = SceneBuilder::named("TriScene");
    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Tri");
    b.attach_mesh(r, mi);
    b.build()
}

/// A triangle scene with a PBR material.
pub fn material_scene() -> Scene {
    let mut b = SceneBuilder::named("MatScene");
    let mut mat = Material::new("RedMat");
    mat.base_color_factor = Vec4::new(0.9, 0.1, 0.1, 1.0);
    mat.roughness_factor = 0.4;
    mat.metallic_factor = 0.2;
    let mi_mat = b.push_material(mat);

    let mut mesh = Mesh::new("ColoredTri");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi_mat))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// A scene with a parent–child Xform hierarchy.
pub fn hierarchy_scene() -> Scene {
    let mut b = SceneBuilder::named("HierScene");
    let root = b.add_root_node("Parent");
    let _child = b.add_child_node(root, "Child");
    b.build()
}

// ── Round-trip helpers ────────────────────────────────────────────────────────

pub fn usda_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::<u8>::new();
    UsdSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .expect("USDA save failed");
    UsdLoader
        .load(&mut Cursor::new(&buf), &LoadOptions::default())
        .expect("USDA load failed")
}

pub fn usda_to_string(scene: &Scene) -> String {
    let mut buf = Vec::<u8>::new();
    UsdSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .expect("save");
    String::from_utf8(buf).expect("utf8")
}
