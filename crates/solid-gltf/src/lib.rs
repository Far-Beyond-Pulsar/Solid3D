//! glTF 2.0 / GLB loader and saver for SolidRS.
//!
//! ## Supported features
//!
//! | Feature | Load | Save |
//! |---------|------|------|
//! | **Encoding** | | |
//! | JSON (`.gltf`) | ✅ | ✅ |
//! | Binary GLB | ✅ | ✅ |
//! | External buffer URIs | ✅ | — |
//! | Base64 data URIs | ✅ | ✅ |
//! | **Geometry** | | |
//! | Positions | ✅ | ✅ |
//! | Normals | ✅ | ✅ |
//! | Tangents | ✅ | ✅ |
//! | UV channels (TEXCOORD_0–7) | ✅ | ✅ |
//! | Vertex colours (COLOR_0) | ✅ | ✅ |
//! | Sparse accessors | ✅ | — |
//! | Morph targets (blend shapes) | ✅ | ✅ |
//! | **Scene graph** | | |
//! | Node hierarchy | ✅ | ✅ |
//! | TRS transforms | ✅ | ✅ |
//! | Matrix transforms | ✅ | — |
//! | **Materials (PBR)** | | |
//! | Base colour + texture | ✅ | ✅ |
//! | Metallic / roughness + texture | ✅ | ✅ |
//! | Normal / occlusion / emissive | ✅ | ✅ |
//! | Explicit specular / IOR + textures | ✅ | ✅ |
//! | Alpha modes | ✅ | ✅ |
//! | Double-sided | ✅ | ✅ |
//! | **Cameras** | | |
//! | Perspective | ✅ | ✅ |
//! | Orthographic | ✅ | ✅ |
//! | **Skinning** | | |
//! | Joints + weights | ✅ | ✅ |
//! | Inverse bind matrices | ✅ | ✅ |
//! | **Animation** | | |
//! | Translation / rotation / scale | ✅ | ✅ |
//! | LINEAR / STEP / CUBICSPLINE | ✅ | ✅ |
//! | Morph target weight animation | ❌ | ❌ |
//! | **Lighting** | | |
//! | KHR_lights_punctual | ✅ | ✅ |
use solid_rs::traits::format::FormatInfo;

mod buffer;
mod convert;
mod document;
pub mod loader;
pub mod saver;

pub use loader::GltfLoader;
pub use saver::GltfSaver;

pub static GLTF_FORMAT: FormatInfo = FormatInfo {
    name: "glTF 2.0",
    id: "gltf",
    extensions: &["gltf", "glb"],
    mime_types: &["model/gltf+json", "model/gltf-binary"],
    can_load: true,
    can_save: true,
    spec_version: Some("2.0"),
};
