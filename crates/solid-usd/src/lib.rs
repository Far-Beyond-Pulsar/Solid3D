//! # solid-usd
//!
//! OpenUSD / USDA loader and saver for [solid-rs](https://crates.io/crates/solid-rs).
//!
//! Provides [`UsdLoader`] and [`UsdSaver`] that can be registered with a
//! `solid_rs::Registry` to add transparent USDA support.
//!
//! ## Supported features
//!
//! | Feature | Load | Save |
//! |---------|------|------|
//! | **Encoding** | | |
//! | USDA (ASCII USD) | ✅ | ✅ |
//! | USDC (binary USD) | ✅ | ❌ |
//! | USDZ (zip container) | ✅ | ❌ |
//! | **Geometry** | | |
//! | Positions (`points`) | ✅ | ✅ |
//! | Normals | ✅ | ✅ |
//! | UV coords (`primvars:st`) | ✅ | ✅ |
//! | Fan-triangulation of quads/n-gons | ✅ | — |
//! | **Scene graph** | | |
//! | Xform hierarchy | ✅ | ✅ |
//! | TRS transforms (`xformOp:translate/rotateXYZ/scale`) | ✅ | ✅ |
//! | **Materials** | | |
//! | UsdPreviewSurface (diffuse, metallic, roughness, emissive) | ✅ | ✅ |
//! | Diffuse texture (`UsdUVTexture`) | ✅ | ✅ |
//! | Material binding (`material:binding`) | ✅ | ✅ |
//! | **Cameras** | | |
//! | Perspective | ✅ | — |
//! | Orthographic | ✅ | — |
//! | **Lights** | | |
//! | PointLight / SphereLight | ✅ | — |
//! | DistantLight | ✅ | — |
//! | SpotLight | ✅ | — |
//! | **Stage metadata** | | |
//! | `upAxis`, `metersPerUnit`, `defaultPrim` | ✅ | ✅ |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use solid_rs::registry::Registry;
//! use solid_usd::{UsdLoader, UsdSaver};
//!
//! let mut registry = Registry::new();
//! registry.register_loader(UsdLoader);
//! registry.register_saver(UsdSaver);
//!
//! let scene = registry.load_file("model.usda").unwrap();
//! println!("Loaded {} meshes", scene.meshes.len());
//!
//! registry.save_file(&scene, "out.usda").unwrap();
//! ```

pub mod document;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod convert;
pub(crate) mod usdc;
pub(crate) mod usdz;
pub mod loader;
pub mod saver;

pub use loader::UsdLoader;
pub use saver::UsdSaver;

use solid_rs::traits::FormatInfo;

/// Metadata for the USD format (all dialects).
pub static USD_FORMAT: FormatInfo = FormatInfo {
    name:         "OpenUSD",
    id:           "usd",
    extensions:   &["usda", "usdc", "usdz", "usd"],
    mime_types:   &["model/vnd.usd", "model/vnd.usdz+zip"],
    can_load:     true,
    can_save:     true,
    spec_version: Some("1.0"),
};
