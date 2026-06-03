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
//! | Binary FBX (v6.1 – v7.7, 32 + 64-bit offsets) | ✅ | ✅ |
//! | ASCII FBX (v7.4) | ✅ | ✅ |
//! | Geometry (positions, normals, UVs) | ✅ | ✅ |
//! | Tangents (`LayerElementTangent`) | ✅ | ✅ |
//! | Vertex colours (`LayerElementColor`) | ✅ | ✅ |
//! | Per-primitive material assignment (`ByPolygon`) | ✅ | ✅ |
//! | Node hierarchy + transforms (TRS) | ✅ | ✅ |
//! | Materials — diffuse, emissive, roughness, metallic, alpha | ✅ | ✅ |
//! | Textures (diffuse + normal map) | ✅ | ✅ |
//! | Cameras (perspective — FOV, near/far) | ✅ | ✅ |
//! | Cameras (orthographic — OrthoZoom, near/far) | ✅ | ✅ |
//! | Lights (point, directional, spot — colour, intensity, cone) | ✅ | ✅ |
//! | Lights (area — size) | ✅ | ✅ |
//! | Skeletal skinning (vertex weights, IBP matrices) | ✅ | ✅ |
//! | Animation clips (translation, rotation, scale) | ✅ | ✅ |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use solid_rs::registry::Registry;
//! use solid_fbx::{FbxLoader, FbxSaver};
//!
//! let mut registry = Registry::new();
//! registry.register_loader(FbxLoader);
//! registry.register_saver(FbxSaver);
//!
//! let scene = registry.load_file("model.fbx").unwrap();
//! println!("Loaded {} meshes", scene.meshes.len());
//!
//! registry.save_file(&scene, "out.fbx").unwrap();
//! ```

pub(crate) mod ascii;
pub(crate) mod binary;
pub(crate) mod convert;
pub mod document;
pub mod loader;
pub mod saver;

pub use loader::FbxLoader;
pub use saver::FbxSaver;

use solid_rs::traits::FormatInfo;

/// Metadata for the FBX format.
pub static FBX_FORMAT: FormatInfo = FormatInfo {
    name: "Autodesk FBX",
    id: "fbx",
    extensions: &["fbx"],
    mime_types: &["application/octet-stream"],
    can_load: true,
    can_save: true,
    spec_version: Some("7.4"),
};
