//! `MdlLoader` — loads Quake MDL model files into a `solid_rs::Scene`.

use std::io::Read;

use solid_rs::prelude::*;

use crate::convert;
use crate::parser;
use crate::MDL_FORMAT;

/// Loader for Quake MDL model files (`.mdl`).
///
/// Parses the binary MDL format, decompresses vertices, looks up normals
/// from the precomputed anorms table, and converts 8-bit indexed textures
/// to RGBA using the Quake colormap.
pub struct MdlLoader;

impl Loader for MdlLoader {
    fn format_info(&self) -> &FormatInfo {
        &MDL_FORMAT
    }

    fn load(&self, reader: &mut dyn ReadSeek, _options: &LoadOptions) -> Result<Scene> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(SolidError::Io)?;

        let mdl = parser::parse_mdl(&data)?;
        convert::mdl_to_scene(&mdl)
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 4];
        if reader.read(&mut buf).unwrap_or(0) < 4 {
            return 0.0;
        }
        // "IDPO" little-endian = 0x4F504449
        if buf == [b'I', b'D', b'P', b'O'] {
            0.95
        } else {
            0.0
        }
    }
}
