//! Legacy DirectX `.x` format support for SolidRS.
//!
//! Supports ASCII `.x` files (`xof ....txt ....`) for both load and save.

pub mod loader;
pub mod saver;

pub use loader::XLoader;
pub use saver::XSaver;

use solid_rs::traits::FormatInfo;

/// Metadata for the DirectX `.x` format.
pub static X_FORMAT: FormatInfo = FormatInfo {
    name: "DirectX X",
    id: "x",
    extensions: &["x"],
    mime_types: &["model/x-directx"],
    can_load: true,
    can_save: true,
    spec_version: Some("0303"),
};
