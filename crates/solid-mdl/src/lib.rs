pub mod constants;
pub mod convert;
pub mod loader;
pub mod parser;
pub mod saver;

pub use loader::MdlLoader;
pub use saver::MdlSaver;

use solid_rs::traits::FormatInfo;

pub static MDL_FORMAT: FormatInfo = FormatInfo {
    name: "Quake MDL",
    id: "mdl",
    extensions: &["mdl"],
    mime_types: &["model/mdl"],
    can_load: true,
    can_save: true,
    spec_version: Some("6"),
};
