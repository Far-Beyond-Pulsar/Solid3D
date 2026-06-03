//! fbx-to-obj — convert a Wavefront FBX file to Wavefront OBJ using SolidRS.
//!
//! Usage:
//!     fbx-to-obj [INPUT.fbx] [OUTPUT.obj]
//!
//! Defaults:
//!     INPUT  = test.fbx   (looked up relative to the workspace root)
//!     OUTPUT = test.obj

use std::path::PathBuf;
use std::time::Instant;

use solid_fbx::{FbxLoader, FbxSaver};
use solid_obj::{ObjLoader, ObjSaver};
use solid_rs::prelude::*;
use solid_rs::registry::Registry;

fn main() {
    // ── Arguments ─────────────────────────────────────────────────────────────
    let mut args = std::env::args().skip(1);
    let input: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("test.fbx"));
    let output: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("obj"));

    println!("solid-fbx → solid-obj converter");
    println!("  input  : {}", input.display());
    println!("  output : {}", output.display());
    println!();

    // ── Build registry ────────────────────────────────────────────────────────
    let mut registry = Registry::new();
    registry.register_loader(FbxLoader);
    registry.register_loader(ObjLoader);
    registry.register_saver(FbxSaver);
    registry.register_saver(ObjSaver);

    // ── Load ──────────────────────────────────────────────────────────────────
    let t0 = Instant::now();

    let load_opts = LoadOptions {
        // Let the OBJ loader resolve MTL files from the same directory as the
        // input file (not used for FBX, but harmless to set).
        base_dir: input.parent().map(|p| p.to_path_buf()),
        triangulate: true,
        ..Default::default()
    };

    println!("Loading …");
    let scene = match registry.load_file_with_options(&input, &load_opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: failed to load '{}': {}", input.display(), e);
            std::process::exit(1);
        }
    };

    let load_ms = t0.elapsed().as_millis();

    // ── Print scene summary ───────────────────────────────────────────────────
    println!("Loaded in {load_ms} ms");
    println!();
    println!(
        "  Scene name : {}",
        if scene.name.is_empty() {
            "(unnamed)"
        } else {
            &scene.name
        }
    );
    println!("  Nodes      : {}", scene.nodes.len());
    println!("  Meshes     : {}", scene.meshes.len());
    println!("  Materials  : {}", scene.materials.len());
    println!("  Textures   : {}", scene.textures.len());
    println!("  Cameras    : {}", scene.cameras.len());
    println!("  Lights     : {}", scene.lights.len());

    let total_verts = scene.total_vertex_count();
    let total_indices = scene.total_index_count();
    println!("  Vertices   : {total_verts}");
    println!(
        "  Indices    : {total_indices}  ({} triangles)",
        total_indices / 3
    );
    println!();

    // Per-mesh summary
    if !scene.meshes.is_empty() {
        println!("  Meshes detail:");
        for (i, mesh) in scene.meshes.iter().enumerate() {
            let idx_count: usize = mesh.primitives.iter().map(|p| p.indices.len()).sum();
            println!(
                "    [{i}] {:30}  {:>6} verts  {:>6} indices  {} primitive(s)",
                mesh.name,
                mesh.vertices.len(),
                idx_count,
                mesh.primitives.len(),
            );
        }
        println!();
    }

    // ── Save ──────────────────────────────────────────────────────────────────
    let t1 = Instant::now();

    println!("Saving  …");
    if let Err(e) = registry.save_file(&scene, &output) {
        eprintln!("ERROR: failed to save '{}': {}", output.display(), e);
        std::process::exit(1);
    }

    let save_ms = t1.elapsed().as_millis();
    let file_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);

    println!(
        "Saved  in {save_ms} ms  ({} bytes → {})",
        file_size,
        output.display()
    );
    println!();
    println!("Done ✓");
}
