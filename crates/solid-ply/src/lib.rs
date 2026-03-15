//! Stanford PLY format loader and saver for SolidRS.
use solid_rs::traits::format::FormatInfo;

mod header;
mod loader;
mod saver;

pub use loader::PlyLoader;
pub use saver::PlySaver;

pub static PLY_FORMAT: FormatInfo = FormatInfo {
    name:         "PLY",
    id:           "ply",
    extensions:   &["ply"],
    mime_types:   &["model/ply"],
    can_load:     true,
    can_save:     true,
    spec_version: Some("1.0"),
};
