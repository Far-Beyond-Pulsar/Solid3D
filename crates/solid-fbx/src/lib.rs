//! # solid-fbx
//!
//! FBX 3D file format support for [solid-rs](https://crates.io/crates/solid-rs).
//!
//! Provides [`FbxLoader`] and [`FbxSaver`] which can be registered with a
//! `solid_rs::Registry` to add transparent FBX support.
//!
//! ## Supported features
//!
//! | Feature | Load | Save |
//! |---------|------|------|
//! | Binary FBX (v7.2 – v7.7) | ✅ | — |
//! | ASCII FBX (v7.4) | ✅ | ✅ |
//! | Geometry (positions, normals, UVs) | ✅ | ✅ |
//! | Node hierarchy + transforms | ✅ | ✅ |
//! | Materials (diffuse / emissive) | ✅ | ✅ |
//! | Textures (filename) | ✅ | ✅ |
//! | Skinning / Animations | ❌ | ❌ |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use solid_rs::Registry;
//! use solid_fbx::{FbxLoader, FbxSaver};
//!
//! let mut registry = Registry::new();
//! registry.register_loader(std::sync::Arc::new(FbxLoader));
//! registry.register_saver(std::sync::Arc::new(FbxSaver));
//!
//! let scene = registry.load_file("model.fbx", Default::default()).unwrap();
//! println!("Loaded {} meshes", scene.meshes.len());
//!
//! registry.save_file(&scene, "out.fbx", Default::default()).unwrap();
//! ```

pub mod document;
pub(crate) mod binary;
pub(crate) mod ascii;
pub(crate) mod convert;
pub mod loader;
pub mod saver;

pub use loader::FbxLoader;
pub use saver::FbxSaver;

use solid_rs::traits::FormatInfo;

/// Metadata for the FBX format.
pub static FBX_FORMAT: FormatInfo = FormatInfo {
    name:       "Autodesk FBX",
    short_name: "fbx",
    extensions: &["fbx"],
    mime_types: &["application/octet-stream"],
    version:    "7.4",
};
