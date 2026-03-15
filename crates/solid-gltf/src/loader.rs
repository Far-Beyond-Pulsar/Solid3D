//! glTF 2.0 / GLB file loader.

use crate::{convert, document::GltfRoot, GLTF_FORMAT};
use solid_rs::traits::{FormatInfo, LoadOptions, Loader, ReadSeek};
use solid_rs::{Result, SolidError};
use solid_rs::scene::scene::Scene;

pub struct GltfLoader;

impl Loader for GltfLoader {
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

        let base_dir = options.base_dir.as_deref();
        convert::gltf_to_scene(&root, &bin_chunk, base_dir)
    }
}

fn parse_glb(data: &[u8]) -> Result<(GltfRoot, Vec<u8>)> {
    if data.len() < 12 {
        return Err(SolidError::parse("GLB: file too short"));
    }
    let magic   = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());

    if magic != 0x46546C67 {
        return Err(SolidError::parse("GLB: invalid magic"));
    }
    if version != 2 {
        return Err(SolidError::parse(format!("GLB: unsupported version {version}")));
    }

    let mut offset = 12usize;
    let mut json_chunk: Option<&[u8]> = None;
    let mut bin_chunk:  Option<&[u8]> = None;

    while offset + 8 <= data.len() {
        let chunk_len  = u32::from_le_bytes(data[offset..offset+4].try_into().unwrap()) as usize;
        let chunk_type = u32::from_le_bytes(data[offset+4..offset+8].try_into().unwrap());
        let chunk_data = &data[offset+8..offset+8+chunk_len];
        match chunk_type {
            0x4E4F534A => json_chunk = Some(chunk_data), // JSON
            0x004E4942 => bin_chunk  = Some(chunk_data), // BIN\0
            _ => {}
        }
        offset += 8 + chunk_len;
    }

    let json = json_chunk.ok_or_else(|| SolidError::parse("GLB: missing JSON chunk"))?;
    let root: GltfRoot = serde_json::from_slice(json)
        .map_err(|e| SolidError::parse(format!("GLB JSON: {e}")))?;
    Ok((root, bin_chunk.map(|b| b.to_vec()).unwrap_or_default()))
}
