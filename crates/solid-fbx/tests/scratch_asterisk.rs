mod common;
use common::*;

use solid_fbx::{FbxLoader, FbxSaver};
use solid_rs::prelude::*;
use std::io::Cursor;

#[test]
fn scratch_load_asterisk_binary() {
    let bytes = std::fs::read("../../asterisk.fbx").unwrap();
    let mut cursor = Cursor::new(bytes);
    match FbxLoader.load(&mut cursor, &LoadOptions::default()) {
        Ok(scene) => {
            println!(
                "LOADED: meshes={} materials={} nodes={} cameras={} lights={} skins={} animations={}",
                scene.meshes.len(),
                scene.materials.len(),
                scene.nodes.len(),
                scene.cameras.len(),
                scene.lights.len(),
                scene.skins.len(),
                scene.animations.len()
            );
            for m in &scene.meshes {
                println!(
                    "  mesh '{}': {} verts, {} prims, {} tris",
                    m.name,
                    m.vertices.len(),
                    m.primitives.len(),
                    m.primitives.iter().map(|p| p.indices.len() / 3).sum::<usize>()
                );
            }
        }
        Err(e) => println!("LOAD FAILED: {e:?}"),
    }
}
