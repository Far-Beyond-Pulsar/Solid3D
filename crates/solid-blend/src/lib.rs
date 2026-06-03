//! Blender `.blend` format support for SolidRS.
//!
//! This crate uses a Blender CLI bridge:
//! - load: `.blend` -> temporary `.glb` -> SolidRS scene
//! - save: scene -> temporary `.glb` -> `.blend`
//!
//! Blender executable path can be overridden with `BLENDER_BIN`.

mod bridge;
pub mod loader;
pub mod saver;

pub use loader::BlendLoader;
pub use saver::BlendSaver;

use solid_rs::traits::FormatInfo;

/// Metadata for Blender `.blend`.
pub static BLEND_FORMAT: FormatInfo = FormatInfo {
    name: "Blender",
    id: "blend",
    extensions: &["blend"],
    mime_types: &["application/x-blender"],
    can_load: true,
    can_save: true,
    spec_version: None,
};
