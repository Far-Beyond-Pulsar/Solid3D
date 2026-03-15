//! The [`Loader`] trait and [`LoadOptions`].

use std::io::{Read, Seek};
use std::path::PathBuf;

use crate::error::Result;
use crate::scene::scene::Scene;
use crate::traits::FormatInfo;

/// Combined `Read + Seek` supertrait — blanket-implemented for all `T: Read + Seek`.
///
/// This exists solely so that `Loader::load` can accept a `&mut dyn ReadSeek`
/// without making the `Loader` trait non-dyn-compatible.
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// Options that control how a scene is parsed.
///
/// All fields have sensible defaults via [`Default`]; loaders should honour
/// as many of these as is practical and silently ignore options they do not
/// support.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Generate smooth normals for meshes that have none in the file.
    pub generate_normals: bool,

    /// Triangulate non-triangle polygons (quads, n-gons).
    pub triangulate: bool,

    /// Weld duplicate vertices (same position + attributes) into one.
    pub merge_vertices: bool,

    /// Flip the V (vertical) texture coordinate: `v' = 1 − v`.
    /// Needed when converting between top-left and bottom-left UV origins.
    pub flip_uv_v: bool,

    /// Downscale textures to at most this dimension on their longest axis.
    /// `None` = no limit.
    pub max_texture_size: Option<u32>,

    /// Base directory used to resolve relative texture URI paths.
    /// `None` = use the directory of the source file where available.
    pub base_dir: Option<PathBuf>,
}

/// Implemented by format crates to parse a byte stream into a [`Scene`].
///
/// # Implementing a Loader
///
/// ```ignore
/// use solid_rs::prelude::*;
/// use std::io::{Read, Seek};
///
/// pub struct MyFmtLoader;
///
/// static FMT: FormatInfo = FormatInfo {
///     name:         "My Format",
///     id:           "myfmt",
///     extensions:   &["myfmt"],
///     mime_types:   &["model/x-myfmt"],
///     can_load:     true,
///     can_save:     false,
///     spec_version: None,
/// };
///
/// impl Loader for MyFmtLoader {
///     fn load(
///         &self,
///         reader: &mut dyn ReadSeek,
///         options: &LoadOptions,
///     ) -> Result<Scene> {
///         let mut builder = SceneBuilder::new();
///         // … parse `reader`, populate `builder` …
///         Ok(builder.build())
///     }
///
///     fn format_info(&self) -> &FormatInfo { &FMT }
/// }
/// ```
pub trait Loader: Send + Sync + 'static {
    /// Parses data from `reader` and returns a fully populated [`Scene`].
    ///
    /// `reader` implements both [`Read`] and [`Seek`] via [`ReadSeek`] so
    /// loaders can inspect magic bytes, rewind, or jump to offsets.
    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        options: &LoadOptions,
    ) -> Result<Scene>;

    /// Returns static metadata describing the format this loader handles.
    fn format_info(&self) -> &FormatInfo;

    /// Optional magic-byte probe.
    ///
    /// The registry calls this to auto-detect the format when the file
    /// extension is ambiguous.  Implementations should read as few bytes
    /// as possible and leave the reader position unchanged.
    ///
    /// Returns a confidence score in `[0.0, 1.0]`; `0.0` means "cannot
    /// determine" and `1.0` means "definitely this format".
    fn detect(&self, _reader: &mut dyn Read) -> f32 {
        0.0
    }
}
