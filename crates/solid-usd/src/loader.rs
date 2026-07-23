//! USD family loader — dispatches USDA (text), USDC (binary Crate), and USDZ (ZIP).

use std::io::{Cursor, Read};

use solid_rs::{
    scene::scene::Scene,
    traits::{FormatInfo, LoadOptions, Loader, ReadSeek},
    SolidError,
};

use crate::{convert, parser, usdc, usdz, USD_FORMAT};

pub struct UsdLoader;

impl Loader for UsdLoader {
    /// USD-specific import options, extending the common set. Fields not yet
    /// honoured by the loader are ignored (per the `LoadOptions` contract) and
    /// may be consumed by the host during conversion.
    #[cfg(feature = "configurator")]
    fn options_schema(&self) -> solid_rs::configurator::OptionsSchema {
        use solid_rs::configurator::{OptionField, OptionsSchema};
        OptionsSchema::base_load_options()
            .with(OptionField::choice(
                "up_axis",
                "Up axis",
                "Stage up-axis to assume when not declared (converted to engine Y-up).",
                "Y",
                &["Y", "Z"],
            ))
            .with(OptionField::bool(
                "import_materials",
                "Import materials",
                "Import USD material bindings if present.",
                true,
            ))
    }

    fn format_info(&self) -> &'static FormatInfo {
        &USD_FORMAT
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 10];
        let n = reader.read(&mut buf).unwrap_or(0);
        let s = &buf[..n];
        if s.starts_with(b"#usda ") {
            return 0.95;
        }
        if s.starts_with(b"PXR-USDC") {
            return 0.90;
        }
        if s.starts_with(b"PK\x03\x04") {
            return 0.85;
        } // USDZ
        0.0
    }

    fn load(&self, reader: &mut dyn ReadSeek, _options: &LoadOptions) -> Result<Scene, SolidError> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(SolidError::Io)?;

        // --- USDC binary ---
        if data.starts_with(b"PXR-USDC") {
            let doc = usdc::read(&data)?;
            return convert::doc_to_scene(&doc);
        }

        // --- USDZ ZIP container ---
        if data.starts_with(b"PK\x03\x04") {
            let doc = usdz::read(Cursor::new(&data))?;
            return convert::doc_to_scene(&doc);
        }

        // --- USDA text ---
        let src = std::str::from_utf8(&data)
            .map_err(|e| SolidError::parse(format!("USD file is not valid UTF-8: {e}")))?;
        let doc = parser::parse(src)?;
        convert::doc_to_scene(&doc)
    }
}
