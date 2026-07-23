//! # SolidRS
//!
//! A generic 3D model loading and saving library providing core traits,
//! scene primitives, and a format registry — analogous to how `serde`
//! provides serialisation infrastructure while format crates like `serde_json`
//! handle the specifics.
//!
//! ## Crate Ecosystem
//!
//! | Crate         | Role                                          |
//! |:------------- |:--------------------------------------------- |
//! | `solid-rs`    | Core types, traits, and registry (this crate) |
//! | `solid-obj`   | Wavefront OBJ loader / saver                  |
//! | `solid-fbx`   | Autodesk FBX loader / saver                   |
//! | `solid-gltf`  | glTF 2.0 / GLB loader / saver                 |
//! | `solid-usd`   | OpenUSD / USDA / USDC loader / saver          |
//! | `solid-stl`   | Stereolithography STL loader / saver          |
//! | `solid-ply`   | Stanford PLY loader / saver                   |
//! | `solid-blend` | Blender `.blend` loader / saver               |
//! | `solid-x`     | Legacy DirectX `.x` loader / saver            |
//!
//! ## Quick Start
//!
//! ```toml
//! # Cargo.toml
//! [dependencies]
//! solid-rs  = "0.1"
//! solid-obj = "0.1"   # add whichever format crates you need
//! ```
//!
//! ```rust,no_run
//! use solid_rs::prelude::*;
//!
//! fn main() -> Result<()> {
//!     let mut registry = Registry::new();
//!     // registry.register_loader(solid_obj::ObjLoader::default());
//!     // let scene = registry.load_file("model.obj")?;
//!     // println!("Loaded {} meshes", scene.meshes.len());
//!     Ok(())
//! }
//! ```
//!
//! ## Design Overview
//!
//! ```text
//!  ┌─────────────────────────────────────────────┐
//!  │                  solid-rs                   │
//!  │  Scene ─ Node ─ Mesh ─ Material ─ Texture   │
//!  │  Loader trait ─ Saver trait ─ Registry      │
//!  └──────────────────┬──────────────────────────┘
//!                     │ implements
//!       ┌─────────────┼─────────────┐
//!       ▼             ▼             ▼
//!  solid-obj      solid-gltf    solid-fbx  …
//! ```
//!
//! ## Feature Flags
//!
//! - **`configurator`** (off by default): adds the [`configurator`] module plus
//!   [`Loader::options_schema`](crate::traits::Loader::options_schema) and
//!   [`Loader::load_configured`](crate::traits::Loader::load_configured), a small
//!   runtime schema/value system so a host can render an import configurator UI
//!   from a loader's advertised options. Format crates expose their own flags too.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod builder;
#[cfg(feature = "configurator")]
pub mod configurator;
pub mod error;
pub mod extensions;
pub mod geometry;
pub mod prelude;
pub mod registry;
pub mod scene;
pub mod traits;
pub mod value;

pub use error::{Result, SolidError};
pub use scene::scene::Scene;

/// Re-export of the [`glam`] math library so downstream format crates and
/// users do not need a separate version-aligned dependency.
pub use glam;
