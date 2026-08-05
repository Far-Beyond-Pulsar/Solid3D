use std::io::Read;
use solid_rs::scene::Scene;
use solid_rs::traits::{FormatInfo, LoadOptions, Loader, ReadSeek};

use crate::convert::{package_to_scene_from_uasset, UnrealConvertConfig};

pub static UNREAL_FORMAT: FormatInfo = FormatInfo {
    name: "Unreal Engine Package",
    id: "unreal",
    extensions: &["uasset", "umap", "uexp", "ubulk"],
    mime_types: &["application/x-ue-package"],
    can_load: true,
    can_save: false,
    spec_version: Some("UE 4.27, 5.0-5.5"),
};

pub struct UnrealLoader;

/// Map [`LoadOptions`] onto the Unreal converter config. Each option is wired
/// independently so enabling one never has hidden side effects.
fn convert_config(options: &LoadOptions) -> UnrealConvertConfig {
    UnrealConvertConfig {
        merge_meshes: options.merge_meshes,
        embed_textures: true,
        max_texture_size: options.max_texture_size.unwrap_or(2048),
        flatten_hierarchy: true,
        generate_normals: options.generate_normals,
        triangulate: options.triangulate,
    }
}

/// Shared load path used by [`Loader::load`] and [`Loader::load_configured`].
fn load_with_config(
    reader: &mut dyn ReadSeek,
    config: &UnrealConvertConfig,
) -> solid_rs::error::Result<Scene> {
    let scene = package_to_scene_from_uasset(reader, config)
        .map_err(|e| solid_rs::error::SolidError::parse(e.to_string()))?;
    Ok(scene)
}

impl Loader for UnrealLoader {
    /// Unreal-specific import options, extending the common set.
    #[cfg(feature = "configurator")]
    fn options_schema(&self) -> solid_rs::configurator::OptionsSchema {
        use solid_rs::configurator::{OptionField, OptionsSchema};
        OptionsSchema::base_load_options()
            .with(OptionField::bool(
                "flatten_hierarchy",
                "Flatten hierarchy",
                "Flatten the actor hierarchy into a flat list of meshes.",
                true,
            ))
            .with(OptionField::bool(
                "embed_textures",
                "Embed textures",
                "Copy texture data into the scene instead of referencing files.",
                true,
            ))
    }

    #[cfg(feature = "configurator")]
    fn load_configured(
        &self,
        reader: &mut dyn ReadSeek,
        values: &solid_rs::configurator::OptionValues,
    ) -> solid_rs::error::Result<Scene> {
        let opts = values.to_load_options();
        let mut config = convert_config(&opts);
        config.flatten_hierarchy = values.bool_or("flatten_hierarchy", config.flatten_hierarchy);
        config.embed_textures = values.bool_or("embed_textures", config.embed_textures);
        load_with_config(reader, &config)
    }

    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        options: &LoadOptions,
    ) -> solid_rs::error::Result<Scene> {
        load_with_config(reader, &convert_config(options))
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

#[cfg(test)]
mod tests {
    use super::*;
    use solid_rs::traits::LoadOptions;

    #[test]
    fn merge_meshes_wired_independently_of_triangulate() {
        // #28: triangulate must NOT imply merge_meshes.
        let mut o = LoadOptions {
            triangulate: true,
            ..LoadOptions::default()
        };
        assert!(!convert_config(&o).merge_meshes);
        assert!(convert_config(&o).triangulate);

        o.merge_meshes = true;
        assert!(convert_config(&o).merge_meshes);
    }

    #[test]
    fn max_texture_size_maps_through() {
        let mut o = LoadOptions::default();
        o.max_texture_size = Some(1024);
        assert_eq!(convert_config(&o).max_texture_size, 1024);
        o.max_texture_size = None;
        assert_eq!(convert_config(&o).max_texture_size, 2048);
    }
}
