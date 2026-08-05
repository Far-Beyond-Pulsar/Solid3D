//! glTF 2.0 / GLB file loader.

use crate::{convert, document::GltfRoot, GLTF_FORMAT};
use solid_rs::scene::scene::Scene;
use solid_rs::traits::{FormatInfo, LoadOptions, Loader, ReadSeek};
use solid_rs::{Result, SolidError};

pub struct GltfLoader;

impl Loader for GltfLoader {
    /// glTF-specific import options, extending the common set. Fields not yet
    /// honoured by the loader are ignored (per the `LoadOptions` contract) and
    /// may be consumed by the host during conversion.
    #[cfg(feature = "configurator")]
    fn options_schema(&self) -> solid_rs::configurator::OptionsSchema {
        use solid_rs::configurator::{OptionField, OptionsSchema};
        OptionsSchema::base_load_options()
            .with(OptionField::float(
                "import_scale",
                "Import scale",
                "Uniform scale applied to the imported scene.",
                1.0,
                Some(0.0001),
                Some(10000.0),
                Some(0.01),
            ))
            .with(OptionField::bool(
                "import_animations",
                "Import animations",
                "Import animation channels if present.",
                true,
            ))
            .with(OptionField::bool(
                "import_cameras",
                "Import cameras",
                "Import camera nodes if present.",
                true,
            ))
    }

    fn format_info(&self) -> &'static FormatInfo {
        &GLTF_FORMAT
    }

    fn load(&self, reader: &mut dyn ReadSeek, options: &LoadOptions) -> Result<Scene> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(SolidError::Io)?;

        let (root, bin_chunk) = if data.starts_with(b"glTF") {
            parse_glb(&data)?
        } else {
            let root: GltfRoot = serde_json::from_slice(&data)
                .map_err(|e| SolidError::parse(format!("glTF JSON: {e}")))?;
            (root, vec![])
        };

        enforce_supported_extensions(&root)?;

        let base_dir = options.base_dir.as_deref();
        convert::gltf_to_scene(&root, &bin_chunk, base_dir)
    }
}

/// glTF 2.0 requires loaders to error when `extensionsRequired` lists an
/// extension they cannot fully support — silently loading a mesh as empty
/// geometry would mask real data loss (e.g. Draco-compressed attributes).
fn enforce_supported_extensions(root: &GltfRoot) -> Result<()> {
    const SUPPORTED_REQUIRED: &[&str] = &[
        "KHR_lights_punctual",
        "KHR_materials_specular",
        "KHR_materials_ior",
    ];
    let unsupported: Vec<&str> = root
        .extensions_required
        .iter()
        .filter(|ext| !SUPPORTED_REQUIRED.contains(&ext.as_str()))
        .map(String::as_str)
        .collect();
    if unsupported.is_empty() {
        return Ok(());
    }
    Err(SolidError::unsupported(format!(
        "glTF requires extensions that solid-gltf cannot load: {}",
        unsupported.join(", ")
    )))
}

fn parse_glb(data: &[u8]) -> Result<(GltfRoot, Vec<u8>)> {
    if data.len() < 12 {
        return Err(SolidError::parse("GLB: file too short"));
    }
    let magic = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());

    if magic != 0x46546C67 {
        return Err(SolidError::parse("GLB: invalid magic"));
    }
    if version != 2 {
        return Err(SolidError::parse(format!(
            "GLB: unsupported version {version}"
        )));
    }

    let mut offset = 12usize;
    let mut json_chunk: Option<&[u8]> = None;
    let mut bin_chunk: Option<&[u8]> = None;

    while offset + 8 <= data.len() {
        let chunk_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_type = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
        let chunk_start = offset + 8;
        let chunk_end = chunk_start
            .checked_add(chunk_len)
            .ok_or_else(|| SolidError::parse("GLB: chunk length overflow"))?;
        if chunk_end > data.len() {
            return Err(SolidError::parse(format!(
                "GLB: chunk at byte {chunk_start} declares {chunk_len} bytes but only {} remain",
                data.len() - chunk_start
            )));
        }
        let chunk_data = &data[chunk_start..chunk_end];
        match chunk_type {
            0x4E4F534A => json_chunk = Some(chunk_data), // JSON
            0x004E4942 => bin_chunk = Some(chunk_data),  // BIN\0
            _ => {}
        }
        offset = chunk_end;
    }

    let json = json_chunk.ok_or_else(|| SolidError::parse("GLB: missing JSON chunk"))?;
    let root: GltfRoot =
        serde_json::from_slice(json).map_err(|e| SolidError::parse(format!("GLB JSON: {e}")))?;
    Ok((root, bin_chunk.map(|b| b.to_vec()).unwrap_or_default()))
}
