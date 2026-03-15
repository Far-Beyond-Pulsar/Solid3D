//! The [`Saver`] trait and [`SaveOptions`].

use std::io::Write;

use crate::error::Result;
use crate::scene::scene::Scene;
use crate::traits::FormatInfo;

/// Options that control how a scene is serialised.
///
/// All fields have sensible defaults via [`Default`]; savers should honour
/// as many of these as is practical.
#[derive(Debug, Clone, Default)]
pub struct SaveOptions {
    /// Embed textures as inline binary blobs rather than external URI files.
    pub embed_textures: bool,

    /// Produce human-readable (pretty-printed) output where the format
    /// supports it (e.g. JSON-based formats).
    pub pretty_print: bool,

    /// Override the copyright string written into the file header.
    pub copyright: Option<String>,

    /// Override the generator string written into the file header.
    /// Defaults to `"solid-rs <version>"` when `None`.
    pub generator: Option<String>,

    /// Flip the V (vertical) texture coordinate when saving.
    pub flip_uv_v: bool,
}

/// Implemented by format crates to serialise a [`Scene`] to a byte stream.
///
/// # Implementing a Saver
///
/// ```ignore
/// use solid_rs::prelude::*;
/// use std::io::Write;
///
/// pub struct MyFmtSaver;
///
/// static FMT: FormatInfo = FormatInfo {
///     name:         "My Format",
///     id:           "myfmt",
///     extensions:   &["myfmt"],
///     mime_types:   &["model/x-myfmt"],
///     can_load:     false,
///     can_save:     true,
///     spec_version: None,
/// };
///
/// impl Saver for MyFmtSaver {
///     fn save(
///         &self,
///         scene: &Scene,
///         writer: &mut dyn Write,
///         options: &SaveOptions,
///     ) -> Result<()> {
///         // … serialise `scene` to `writer` …
///         Ok(())
///     }
///
///     fn format_info(&self) -> &FormatInfo { &FMT }
/// }
/// ```
pub trait Saver: Send + Sync + 'static {
    /// Serialises `scene` and writes the result to `writer`.
    fn save(
        &self,
        scene: &Scene,
        writer: &mut dyn Write,
        options: &SaveOptions,
    ) -> Result<()>;

    /// Returns static metadata describing the format this saver handles.
    fn format_info(&self) -> &FormatInfo;
}
