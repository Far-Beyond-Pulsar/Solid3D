//! Stanford PLY format loader and saver for SolidRS.
//!
//! ## Supported features
//!
//! | Feature                          | Status |
//! |----------------------------------|--------|
//! | ASCII load                       | ✅     |
//! | Binary little-endian load        | ✅     |
//! | Binary big-endian load           | ✅     |
//! | ASCII save                       | ✅     |
//! | Binary little-endian save        | ✅     |
//! | Binary big-endian save           | ✅     |
//! | Double-precision (`f64`) save    | ✅     |
//! | Point cloud save (no faces)      | ✅     |
//! | Normals, vertex color            | ✅     |
//! | Tangents save                    | ✅     |
//! | Multiple UV channels (0–7)       | ✅     |
//! | N-gon fan triangulation          | ✅     |
//! | All meshes in one file           | ✅     |
use solid_rs::traits::format::FormatInfo;

mod header;
mod loader;
mod saver;

pub use loader::PlyLoader;
pub use saver::PlySaver;

pub static PLY_FORMAT: FormatInfo = FormatInfo {
    name: "PLY",
    id: "ply",
    extensions: &["ply"],
    mime_types: &["model/ply"],
    can_load: true,
    can_save: true,
    spec_version: Some("1.0"),
};
