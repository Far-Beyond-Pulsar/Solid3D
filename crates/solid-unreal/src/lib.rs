// Allow dead_code for public data types — they're exposed as API even if not all
// fields are consumed internally.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_parens)]

//! # solid-unreal
//!
//! Unreal Engine package (`.uasset` / `.umap`) loader for
//! [solid-rs](https://crates.io/crates/solid-rs).
//!
//! Provides [`UnrealLoader`] that can be registered with a
//! `solid_rs::Registry` to load UE package files and convert them
//! to the Solid3D scene model.
//!
//! ## Features
//!
//! | Feature              | Load |
//! |----------------------|------|
//! | **Package parsing**  |      |
//! | UE 4.27 packages     | ✅   |
//! | UE 5.0–5.5 packages  | ✅   |
//! | Binary `.uasset`     | ✅   |
//! | Binary `.umap`       | ✅   |
//! | Name table (ANSI)    | ✅   |
//! | Name table (wide)    | ✅   |
//! | Name table (UE5)     | ✅   |
//! | **Assets**           |      |
//! | UStaticMesh          | ✅   |
//! | UTexture2D           | ✅   |
//! | UMaterial / MIC      | ✅   |
//! | UWorld / ULevel      | ✅   |
//! | **Conversion**       |      |
//! | Merge-to-single-mesh | ✅   |
//! | Texture embedding    | ✅   |
//! | Material PBR mapping | ✅   |
//! | Scene graph          | ✅   |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use solid_rs::registry::Registry;
//! use solid_unreal::UnrealLoader;
//!
//! let mut registry = Registry::new();
//! registry.register_loader(UnrealLoader);
//!
//! let scene = registry.load_file("level.umap").unwrap();
//! println!("Loaded {} meshes, {} materials",
//!     scene.meshes.len(), scene.materials.len());
//! ```

pub(crate) mod archive;
pub(crate) mod assets;
pub(crate) mod convert;
pub(crate) mod error;
pub mod loader;
pub(crate) mod package;
pub(crate) mod types;
pub(crate) mod uobject;
pub(crate) mod version;

pub use error::UnrealError;
pub use loader::UnrealLoader;
pub use loader::UNREAL_FORMAT;
pub use package::reader::UPackage;
pub use types::{FName, FNameEntry, ObjectExport, ObjectImport, PackageIndex};
pub use version::{EngineVersion, PackageVersion, UE4Version};
