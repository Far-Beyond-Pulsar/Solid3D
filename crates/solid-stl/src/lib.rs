//! STL (Stereolithography) binary and ASCII loader and saver for SolidRS.
//!
//! ## Supported features
//!
//! | Feature                   | Supported |
//! |---------------------------|-----------|
//! | Binary load               | ✅        |
//! | ASCII load                | ✅        |
//! | Binary save               | ✅        |
//! | ASCII save                | ✅        |
//! | Vertex deduplication      | ✅        |
//! | Smooth vertex normals     | ✅        |
//! | Vertex colors (VisCAM)    | ✅        |
//! | Multiple meshes (binary)  | ✅        |
//! | Multiple meshes (ASCII)   | ✅        |
use solid_rs::traits::format::FormatInfo;
mod loader;
mod saver;
mod parser;
pub use loader::StlLoader;
pub use saver::StlSaver;

pub static STL_FORMAT: FormatInfo = FormatInfo {
    name:         "STL",
    id:           "stl",
    extensions:   &["stl"],
    mime_types:   &["model/stl", "application/sla"],
    can_load:     true,
    can_save:     true,
    spec_version: None,
};
