//! `FbxLoader` — loads binary and ASCII FBX files into a `solid_rs::Scene`.

use std::io::Read;

use solid_rs::prelude::*;
use solid_rs::{Result, SolidError};
use solid_rs::scene::Scene;

use crate::{ascii, binary, convert, FBX_FORMAT};

/// Loader for Autodesk FBX files (binary and ASCII variants).
pub struct FbxLoader;

impl Loader for FbxLoader {
    fn format_info(&self) -> &FormatInfo {
        &FBX_FORMAT
    }

    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        _options: &LoadOptions,
    ) -> Result<Scene> {
        // Detect format variant (binary magic vs ASCII comment header)
        if binary::detect(reader) {
            let doc = binary::parse(reader)?;
            convert::fbx_to_scene(&doc)
        } else if ascii::detect(reader) {
            let doc = ascii::parse(reader)?;
            convert::fbx_to_scene(&doc)
        } else {
            Err(SolidError::parse(
                "file does not appear to be an FBX document \
                 (neither binary magic nor ASCII header found)",
            ))
        }
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        // Read up to 64 bytes for detection without seeking
        let mut buf = [0u8; 64];
        let n = reader.read(&mut buf).unwrap_or(0);
        let slice = &buf[..n];

        // Binary magic is the first 23 bytes
        if slice.len() >= 23 && &slice[..23] == b"Kaydara FBX Binary  \x00\x1a\x00" {
            return 1.0;
        }
        // ASCII FBX starts with `; FBX`
        if slice.starts_with(b"; FBX") || slice.starts_with(b";FBX") {
            return 0.8;
        }
        0.0
    }
}
