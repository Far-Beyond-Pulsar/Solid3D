//! Shared helpers for solid-obj integration tests.

#![allow(dead_code)]

use glam::{Vec2, Vec3, Vec4};
use solid_obj::{ObjLoader, ObjSaver};
use solid_rs::prelude::*;
use solid_rs::scene::Image;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Returns a unique temp directory for each call.
pub fn unique_test_dir() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("solid-obj-test-{}-{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ── Scene factories ───────────────────────────────────────────────────────────

/// 3 vertices with normals, 1 triangle primitive, no material.
pub fn triangle_scene() -> Scene {
    let mut b = SceneBuilder::named("TriangleScene");
    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Triangle");
    b.attach_mesh(r, mi);
    b.build()
}

/// Scene with full PBR material including diffuse/emissive/normal textures.
pub fn material_scene() -> Scene {
    let mut b = SceneBuilder::named("MaterialScene");

    let img_diff = b.push_image(Image::from_uri("diffuse_img", "diffuse.png"));
    let img_emit = b.push_image(Image::from_uri("emissive_img", "emissive.png"));
    let img_norm = b.push_image(Image::from_uri("normal_img", "normal.png"));
    let tex_diff = b.push_texture(Texture::new("diffuse_tex", img_diff));
    let tex_emit = b.push_texture(Texture::new("emissive_tex", img_emit));
    let tex_norm = b.push_texture(Texture::new("normal_tex", img_norm));

    let mut mat = Material::new("PBRMat");
    mat.base_color_factor = Vec4::new(0.8, 0.4, 0.2, 1.0);
    mat.emissive_factor = Vec3::new(0.1, 0.2, 0.3);
    mat.metallic_factor = 0.3;
    mat.roughness_factor = 0.7;
    mat.base_color_texture = Some(TextureRef::new(tex_diff));
    mat.emissive_texture = Some(TextureRef::new(tex_emit));
    mat.normal_texture = Some(TextureRef::new(tex_norm));
    let mat_idx = b.push_material(mat);

    let mut mesh = Mesh::new("Mesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.5, 1.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mat_idx))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Mesh");
    b.attach_mesh(r, mi);
    b.build()
}

/// Two meshes with different solid-color materials.
pub fn multi_mesh_scene() -> Scene {
    let mut b = SceneBuilder::named("MultiMeshScene");
    let mat0 = b.push_material(Material::solid_color("MatA", Vec4::new(1.0, 0.0, 0.0, 1.0)));
    let mat1 = b.push_material(Material::solid_color("MatB", Vec4::new(0.0, 1.0, 0.0, 1.0)));

    let mut mesh0 = Mesh::new("MeshA");
    mesh0.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)),
    ];
    mesh0.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mat0))];

    let mut mesh1 = Mesh::new("MeshB");
    mesh1.vertices = vec![
        Vertex::new(Vec3::new(0.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(-1.0, 0.0, -1.0)),
        Vertex::new(Vec3::new(1.0, 0.0, -1.0)),
    ];
    mesh1.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mat1))];

    let mi0 = b.push_mesh(mesh0);
    let mi1 = b.push_mesh(mesh1);
    let r0 = b.add_root_node("MeshA");
    let r1 = b.add_root_node("MeshB");
    b.attach_mesh(r0, mi0);
    b.attach_mesh(r1, mi1);
    b.build()
}

// ── Round-trip helpers ────────────────────────────────────────────────────────

/// Save a scene to OBJ+MTL on disk, then reload it.
pub fn obj_round_trip(scene: &Scene) -> Scene {
    let dir = unique_test_dir();
    let obj_path = dir.join("scene.obj");
    let mtl_path = dir.join("scene.mtl");

    let mut obj_buf = Vec::new();
    ObjSaver
        .save(scene, &mut obj_buf, &SaveOptions::default())
        .unwrap();
    std::fs::write(&obj_path, &obj_buf).unwrap();

    let mut mtl_buf = Vec::new();
    ObjSaver::save_mtl(scene, &mut mtl_buf).unwrap();
    std::fs::write(&mtl_path, &mtl_buf).unwrap();

    let opts = LoadOptions {
        base_dir: Some(dir),
        ..Default::default()
    };
    let mut f = std::fs::File::open(&obj_path).unwrap();
    ObjLoader.load(&mut f, &opts).unwrap()
}

/// Load OBJ from an in-memory string (no MTL).
pub fn load_obj_str(obj: &str) -> Scene {
    let mut cursor = Cursor::new(obj.as_bytes().to_vec());
    ObjLoader
        .load(&mut cursor, &LoadOptions::default())
        .unwrap()
}

/// Load OBJ + MTL strings via a temp directory.
pub fn load_obj_with_mtl(obj: &str, mtl: &str) -> Scene {
    let dir = unique_test_dir();
    let obj_path = dir.join("scene.obj");
    let mtl_path = dir.join("scene.mtl");
    std::fs::write(&obj_path, obj).unwrap();
    std::fs::write(&mtl_path, mtl).unwrap();

    let opts = LoadOptions {
        base_dir: Some(dir),
        ..Default::default()
    };
    let mut f = std::fs::File::open(&obj_path).unwrap();
    ObjLoader.load(&mut f, &opts).unwrap()
}

/// Save a scene to a Vec<u8> and return the UTF-8 text.
pub fn save_obj_to_string(scene: &Scene) -> String {
    let mut buf = Vec::new();
    ObjSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    String::from_utf8(buf).unwrap()
}

/// Retrieve the URI of the image backing `tex_idx` in `scene`, if any.
pub fn image_uri<'a>(scene: &'a Scene, tex_idx: usize) -> Option<&'a str> {
    let tex = scene.textures.get(tex_idx)?;
    let img = scene.images.get(tex.image_index)?;
    if let solid_rs::scene::ImageSource::Uri(u) = &img.source {
        Some(u.as_str())
    } else {
        None
    }
}
