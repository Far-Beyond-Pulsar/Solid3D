//! USDZ container reader.
//!
//! USDZ is a ZIP archive whose first entry is the *root layer* — a `.usda`
//! or `.usdc` file.  All other entries are assets (textures, etc.) referenced
//! by the USD layer.
//!
//! Reference: <https://openusd.org/release/spec_usdz.html>

use std::io::{Cursor, Read, Seek};
use solid_rs::SolidError;
use crate::document::UsdDoc;

/// Read a USDZ ZIP archive and return the parsed [`UsdDoc`] of its root layer.
pub fn read<R: Read + Seek>(reader: R) -> Result<UsdDoc, SolidError> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| SolidError::parse(format!("USDZ: invalid ZIP: {e}")))?;

    // Find the root layer: the first entry whose name ends with .usda, .usdc, or .usd
    let root_index = (0..archive.len())
        .find(|&i| {
            archive.by_index(i).ok().map_or(false, |f| {
                let n = f.name().to_ascii_lowercase();
                n.ends_with(".usda") || n.ends_with(".usdc") || n.ends_with(".usd")
            })
        })
        .ok_or_else(|| SolidError::parse("USDZ: no USD layer found in archive"))?;

    let mut entry = archive.by_index(root_index)
        .map_err(|e| SolidError::parse(format!("USDZ: cannot open root entry: {e}")))?;

    let name = entry.name().to_ascii_lowercase();
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)
        .map_err(SolidError::Io)?;

    if name.ends_with(".usda") || name.ends_with(".usd") {
        let src = std::str::from_utf8(&buf)
            .map_err(|_| SolidError::parse("USDZ root layer is not valid UTF-8"))?;
        crate::parser::parse(src)
    } else {
        // .usdc binary
        crate::usdc::read(&buf)
    }
}
