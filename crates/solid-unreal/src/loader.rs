use std::io::Read;
use solid_rs::scene::Scene;
use solid_rs::traits::{FormatInfo, LoadOptions, Loader, ReadSeek};

use crate::convert::{package_to_scene_from_uasset, UnrealConvertConfig};

/// Metadata for the Unreal Engine package format.
pub static UNREAL_FORMAT: FormatInfo = FormatInfo {
    name: "Unreal Engine Package",
    id: "unreal",
    extensions: &["uasset", "umap", "uexp", "ubulk"],
    mime_types: &["application/x-ue-package"],
    can_load: true,
    can_save: false,
    spec_version: Some("UE 4.27, 5.0-5.5"),
};

/// Loads Unreal Engine package files (`.uasset`, `.umap`) into a `Scene`.
///
/// The loader first parses the package structure (header, name table,
/// import/export tables), then converts the contained assets to a
/// Solid3D scene graph.
///
/// By default, all meshes are merged into a single mesh and textures
/// are embedded as PNG data. Use `LoadOptions` to control this behavior.
pub struct UnrealLoader;

impl Loader for UnrealLoader {
    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        options: &LoadOptions,
    ) -> solid_rs::error::Result<Scene> {
        // Build conversion config from LoadOptions
        let config = UnrealConvertConfig {
            merge_meshes: options.triangulate,
            embed_textures: true,
            max_texture_size: options.max_texture_size.unwrap_or(2048),
            flatten_hierarchy: true,
            generate_normals: options.generate_normals,
            triangulate: options.triangulate,
        };

        // Use uasset crate for package parsing (handles cooked UE4 correctly)
        let scene = package_to_scene_from_uasset(reader, &config)
            .map_err(|e| solid_rs::error::SolidError::parse(e.to_string()))?;

        Ok(scene)
    }

    fn format_info(&self) -> &FormatInfo {
        &UNREAL_FORMAT
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut magic = [0u8; 4];
        if reader.read_exact(&mut magic).is_ok() {
            let val = u32::from_be_bytes(magic);
            if val == 0x9E2A_83C1 || val == 0xC183_2A9E || val == 0x83C1_2A9E {
                return 0.9;
            }
        }
        0.0
    }
}
