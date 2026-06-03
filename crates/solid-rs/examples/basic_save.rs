//! Demonstrates saving a scene via the format registry.
//!
//! In a real programme you would register a format crate's saver and call
//! `registry.save_file(&scene, "output.obj")`.  This example shows how to
//! wire everything up and prints what *would* be saved.

use glam::{Vec3, Vec4};
use solid_rs::prelude::*;

fn main() -> Result<()> {
    // ── 1. Build a registry ───────────────────────────────────────────────
    let registry = Registry::new();
    // registry.register_saver(solid_obj::ObjSaver::default());

    println!("Registered savers:");
    let mut any = false;
    for info in registry.saver_infos() {
        println!(
            "  [{id}]  {name}  (.{exts})",
            id = info.id,
            name = info.name,
            exts = info.extensions.join(", .")
        );
        any = true;
    }
    if !any {
        println!("  (none — add a format crate)");
    }

    // ── 2. Build a scene ──────────────────────────────────────────────────
    let scene = build_scene();

    // ── 3. Save via registry (no-op until a saver is registered) ─────────
    // let opts = SaveOptions { embed_textures: true, pretty_print: true, ..Default::default() };
    // registry.save_file_with_options(&scene, "output.obj", &opts)?;

    println!("\nScene \"{}\" ready for export:", scene.name);
    println!("  {} mesh(es)", scene.meshes.len());
    println!("  {} material(s)", scene.materials.len());

    for mesh in &scene.meshes {
        println!(
            "  Mesh \"{}\" — {} vertices, {} primitives",
            mesh.name,
            mesh.vertex_count(),
            mesh.primitives.len()
        );
    }

    for mat in &scene.materials {
        println!(
            "  Material \"{}\" — base_color {:?}",
            mat.name, mat.base_color_factor
        );
    }

    Ok(())
}

fn build_scene() -> solid_rs::Scene {
    let mut b = SceneBuilder::named("Export Scene");

    // Material
    let mat = Material::solid_color("Red", Vec4::new(0.8, 0.1, 0.1, 1.0));
    let mat_idx = b.push_material(mat);

    // Mesh: a quad made of two triangles.
    let mut quad = Mesh::new("Quad");
    quad.vertices = vec![
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, 1.0, 0.0)).with_normal(Vec3::Z),
    ];
    quad.primitives = vec![Primitive::triangles(vec![0, 1, 2, 0, 2, 3], Some(mat_idx))];
    let quad_idx = b.push_mesh(quad);

    let root = b.add_root_node("Root");
    b.attach_mesh(root, quad_idx);

    b.build()
}
