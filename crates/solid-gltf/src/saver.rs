//! glTF 2.0 / GLB file saver.

use crate::{convert, document::GltfRoot, GLTF_FORMAT};
use solid_rs::traits::{FormatInfo, SaveOptions, Saver};
use solid_rs::{Result, SolidError};
use solid_rs::scene::scene::Scene;
use std::io::Write;

pub struct GltfSaver;

impl Saver for GltfSaver {
    fn format_info(&self) -> &'static FormatInfo {
        &GLTF_FORMAT
    }

    /// Saves the scene as glTF JSON with the binary buffer embedded as a
    /// base64 data URI.  Set `options.pretty_print = false` to get compact
    /// JSON; the default is `false` (compact), pass `true` for human-readable.
    fn save(&self, scene: &Scene, writer: &mut dyn Write, options: &SaveOptions) -> Result<()> {
        let (mut root, bin) = convert::scene_to_gltf(scene)?;

        // Embed the binary buffer as a base64 data URI.
        if !bin.is_empty() {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bin);
            let uri = format!("data:application/octet-stream;base64,{b64}");
            if let Some(buf) = root.buffers.first_mut() {
                buf.uri = Some(uri);
            }
        }

        let json = if options.pretty_print {
            serde_json::to_string_pretty(&root)
        } else {
            serde_json::to_string(&root)
        }
        .map_err(|e| SolidError::parse(format!("glTF serialise: {e}")))?;

        writer.write_all(json.as_bytes()).map_err(SolidError::Io)
    }
}

impl GltfSaver {
    /// Serialise the scene directly as a GLB binary container.
    pub fn save_glb(&self, scene: &Scene, writer: &mut dyn Write) -> Result<()> {
        let (root, bin) = convert::scene_to_gltf(scene)?;
        write_glb(&root, &bin, writer)
    }
}

fn write_glb(root: &GltfRoot, bin: &[u8], writer: &mut dyn Write) -> Result<()> {
    let json = serde_json::to_string(root)
        .map_err(|e| SolidError::parse(format!("GLB JSON: {e}")))?;

    // Pad JSON chunk to 4-byte boundary with spaces (0x20).
    let json_bytes = json.as_bytes();
    let json_pad = (4 - json_bytes.len() % 4) % 4;
    let json_len = json_bytes.len() + json_pad;

    // Pad BIN chunk to 4-byte boundary with zeros.
    let bin_pad = (4 - bin.len() % 4) % 4;
    let bin_len = if bin.is_empty() { 0 } else { bin.len() + bin_pad };

    let total = 12
        + 8 + json_len
        + if bin_len > 0 { 8 + bin_len } else { 0 };

    // Header
    writer.write_all(&0x46546C67u32.to_le_bytes()).map_err(SolidError::Io)?; // "glTF"
    writer.write_all(&2u32.to_le_bytes()).map_err(SolidError::Io)?;          // version 2
    writer.write_all(&(total as u32).to_le_bytes()).map_err(SolidError::Io)?;

    // JSON chunk
    writer.write_all(&(json_len as u32).to_le_bytes()).map_err(SolidError::Io)?;
    writer.write_all(&0x4E4F534Au32.to_le_bytes()).map_err(SolidError::Io)?; // "JSON"
    writer.write_all(json_bytes).map_err(SolidError::Io)?;
    writer.write_all(&vec![0x20u8; json_pad]).map_err(SolidError::Io)?;

    // BIN chunk (optional)
    if bin_len > 0 {
        writer.write_all(&(bin_len as u32).to_le_bytes()).map_err(SolidError::Io)?;
        writer.write_all(&0x004E4942u32.to_le_bytes()).map_err(SolidError::Io)?; // "BIN\0"
        writer.write_all(bin).map_err(SolidError::Io)?;
        writer.write_all(&vec![0u8; bin_pad]).map_err(SolidError::Io)?;
    }

    Ok(())
}
