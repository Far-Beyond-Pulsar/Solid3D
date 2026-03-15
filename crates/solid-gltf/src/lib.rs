//! glTF 2.0 / GLB loader and saver for SolidRS.
use solid_rs::traits::format::FormatInfo;

mod buffer;
mod convert;
mod document;
pub mod loader;
pub mod saver;

pub use loader::GltfLoader;
pub use saver::GltfSaver;

pub static GLTF_FORMAT: FormatInfo = FormatInfo {
    name:         "glTF 2.0",
    id:           "gltf",
    extensions:   &["gltf", "glb"],
    mime_types:   &["model/gltf+json", "model/gltf-binary"],
    can_load:     true,
    can_save:     true,
    spec_version: Some("2.0"),
};
