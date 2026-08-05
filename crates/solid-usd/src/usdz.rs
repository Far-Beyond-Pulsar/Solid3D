//! USDZ container reader.
//!
//! USDZ is a ZIP archive whose first entry is the *root layer* — a `.usda`
//! or `.usdc` file.  All other entries are assets (textures, etc.) referenced
//! by the USD layer.
//!
//! Reference: <https://openusd.org/release/spec_usdz.html>

use crate::document::UsdDoc;
use solid_rs::SolidError;
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};

/// A parsed USDZ container: the root layer plus the embedded asset files
/// (textures etc.) that the layer references.
pub struct UsdzFile {
    /// The parsed root layer document.
    pub doc: UsdDoc,
    /// Asset bytes keyed by their normalized path inside the archive
    /// (`(mime_type, data)`).
    pub assets: HashMap<String, (String, Vec<u8>)>,
}

/// Read a USDZ ZIP archive, returning its root layer and embedded assets.
pub fn read<R: Read + Seek>(reader: R) -> Result<UsdzFile, SolidError> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| SolidError::parse(format!("USDZ: invalid ZIP: {e}")))?;

    // Find the root layer: the first entry whose name ends with .usda, .usdc, or .usd
    let root_index = {
        let mut root = None;
        for i in 0..archive.len() {
            let f = archive
                .by_index(i)
                .map_err(|e| SolidError::parse(format!("USDZ: invalid ZIP entry: {e}")))?;
            let n = f.name().to_ascii_lowercase();
            if n.ends_with(".usda") || n.ends_with(".usdc") || n.ends_with(".usd") {
                root = Some(i);
                break;
            }
        }
        root.ok_or_else(|| SolidError::parse("USDZ: no USD layer found in archive"))?
    };

    // Read the root layer, then drop the borrow before scanning the rest.
    let (name, buf) = {
        let mut entry = archive
            .by_index(root_index)
            .map_err(|e| SolidError::parse(format!("USDZ: cannot open root entry: {e}")))?;
        let name = entry.name().to_ascii_lowercase();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(SolidError::Io)?;
        (name, buf)
    };

    let doc = if name.ends_with(".usda") || name.ends_with(".usd") {
        let src = std::str::from_utf8(&buf)
            .map_err(|_| SolidError::parse("USDZ root layer is not valid UTF-8"))?;
        crate::parser::parse(src)?
    } else {
        // .usdc binary
        crate::usdc::read(&buf)?
    };

    // Extract every non-root entry as an embedded asset.
    let mut assets: HashMap<String, (String, Vec<u8>)> = HashMap::new();
    for i in 0..archive.len() {
        if i == root_index {
            continue;
        }
        let mut f = archive
            .by_index(i)
            .map_err(|e| SolidError::parse(format!("USDZ: cannot open entry {i}: {e}")))?;
        let raw_name = f.name().to_string();
        if raw_name.ends_with('/') {
            continue; // directory entry
        }
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).map_err(SolidError::Io)?;
        assets.insert(normalize_asset_path(&raw_name), (mime_from_path(&raw_name), bytes));
    }

    Ok(UsdzFile { doc, assets })
}

/// Normalize a USD asset path so it matches the corresponding ZIP entry name
/// (strip leading `./` and `/`).
pub(crate) fn normalize_asset_path(p: &str) -> String {
    p.trim_start_matches(['/', '.']).to_string()
}

/// Guess the MIME type of an asset from its file extension.
fn mime_from_path(p: &str) -> String {
    let ext = p.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tga" => "image/x-tga",
        "tiff" | "tif" => "image/tiff",
        "exr" => "image/x-exr",
        _ => "application/octet-stream",
    }
    .to_string()
}
