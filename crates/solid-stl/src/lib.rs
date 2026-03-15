//! STL (Stereolithography) binary and ASCII loader and saver for SolidRS.
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
