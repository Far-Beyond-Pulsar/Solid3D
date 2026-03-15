//! # solid-obj
//!
//! Wavefront OBJ/MTL 3D format support for [solid-rs](https://crates.io/crates/solid-rs).
//!
//! Provides [`ObjLoader`] and [`ObjSaver`] for reading and writing `.obj`
//! files, plus a companion MTL parser for `.mtl` material libraries.
//!
//! ## Supported features
//!
//! | Feature | Load | Save |
//! |---------|------|------|
//! | Vertex positions (`v`) | ✅ | ✅ |
//! | Normals (`vn`) | ✅ | ✅ |
//! | UV coords (`vt`) | ✅ | ✅ |
//! | Objects & groups (`o`, `g`) | ✅ | ✅ |
//! | Material refs (`usemtl`) | ✅ | ✅ |
//! | MTL library (`mtllib`) | ✅ | ✅ |
//! | Diffuse / emissive / alpha | ✅ | ✅ |
//! | Texture maps (`map_Kd`, `map_bump`, …) | ✅ | ✅ |
//! | N-gon fan triangulation | ✅ | — |
//! | Negative indices | ✅ | — |
//! | Skinning / Animations | ❌ | ❌ |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use solid_rs::registry::Registry;
//! use solid_obj::{ObjLoader, ObjSaver};
//!
//! let mut registry = Registry::new();
//! registry.register_loader(ObjLoader);
//! registry.register_saver(ObjSaver);
//!
//! // Load — MTL is resolved automatically if base_dir is set in LoadOptions
//! let scene = registry.load_file("mesh.obj").unwrap();
//! println!("Loaded {} meshes", scene.meshes.len());
//!
//! // Save
//! registry.save_file(&scene, "out.obj").unwrap();
//! ```
//!
//! ## Loading with MTL materials
//!
//! ```rust,no_run
//! use solid_rs::prelude::*;
//! use solid_obj::ObjLoader;
//! use std::path::PathBuf;
//!
//! let loader = ObjLoader;
//! let opts = LoadOptions { base_dir: Some(PathBuf::from("assets/")), ..Default::default() };
//! let mut file = std::fs::File::open("assets/model.obj").unwrap();
//! let scene = loader.load(&mut file, &opts).unwrap();
//! ```

pub mod parser;
pub mod convert;
pub mod loader;
pub mod saver;

pub use loader::ObjLoader;
pub use saver::ObjSaver;

use solid_rs::traits::FormatInfo;

/// Metadata for the Wavefront OBJ format.
pub static OBJ_FORMAT: FormatInfo = FormatInfo {
    name:         "Wavefront OBJ",
    id:           "obj",
    extensions:   &["obj"],
    mime_types:   &["model/obj", "text/plain"],
    can_load:     true,
    can_save:     true,
    spec_version: None,
};
