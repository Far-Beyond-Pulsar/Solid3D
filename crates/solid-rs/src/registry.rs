//! The format registry — dynamic loader/saver selection by extension or MIME type.
//!
//! [`Registry`] is the primary entry-point for applications that want to load
//! or save 3D files without hard-coding a format crate.
//!
//! # Example
//!
//! ```rust,no_run
//! use solid_rs::registry::Registry;
//! use solid_rs::traits::LoadOptions;
//!
//! // Assume `solid_obj` crate is available:
//! // use solid_obj::ObjLoader;
//!
//! let mut registry = Registry::new();
//! // registry.register_loader(ObjLoader::default());
//!
//! // let scene = registry.load_file("model.obj").unwrap();
//! // println!("meshes: {}", scene.meshes.len());
//! ```

use std::io::{Read, Seek};
use std::path::Path;
use std::sync::Arc;

use crate::error::{Result, SolidError};
use crate::scene::scene::Scene;
use crate::traits::{FormatInfo, LoadOptions, Loader, ReadSeek, SaveOptions, Saver};

/// Dynamic registry of [`Loader`] and [`Saver`] implementations.
///
/// Format crates register themselves at runtime; the registry then selects the
/// correct implementation based on file extension, MIME type, or magic bytes.
#[derive(Default)]
pub struct Registry {
    loaders: Vec<Arc<dyn Loader>>,
    savers:  Vec<Arc<dyn Saver>>,
}

impl Registry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    // ── Registration ─────────────────────────────────────────────────────────

    /// Registers a loader implementation.  Returns `&mut self` for chaining.
    pub fn register_loader(&mut self, loader: impl Loader) -> &mut Self {
        self.loaders.push(Arc::new(loader));
        self
    }

    /// Registers a saver implementation.  Returns `&mut self` for chaining.
    pub fn register_saver(&mut self, saver: impl Saver) -> &mut Self {
        self.savers.push(Arc::new(saver));
        self
    }

    // ── Lookup ───────────────────────────────────────────────────────────────

    /// Finds a registered loader by its short format ID (e.g. `"obj"`).
    pub fn loader_by_id(&self, id: &str) -> Option<&dyn Loader> {
        self.loaders
            .iter()
            .find(|l| l.format_info().id.eq_ignore_ascii_case(id))
            .map(Arc::as_ref)
    }

    /// Finds a registered loader by file extension (without leading dot).
    pub fn loader_for_extension(&self, ext: &str) -> Option<&dyn Loader> {
        self.loaders
            .iter()
            .find(|l| l.format_info().matches_extension(ext))
            .map(Arc::as_ref)
    }

    /// Finds a registered loader by MIME type.
    pub fn loader_for_mime(&self, mime: &str) -> Option<&dyn Loader> {
        self.loaders
            .iter()
            .find(|l| l.format_info().matches_mime(mime))
            .map(Arc::as_ref)
    }

    /// Finds a registered saver by its short format ID.
    pub fn saver_by_id(&self, id: &str) -> Option<&dyn Saver> {
        self.savers
            .iter()
            .find(|s| s.format_info().id.eq_ignore_ascii_case(id))
            .map(Arc::as_ref)
    }

    /// Finds a registered saver by file extension (without leading dot).
    pub fn saver_for_extension(&self, ext: &str) -> Option<&dyn Saver> {
        self.savers
            .iter()
            .find(|s| s.format_info().matches_extension(ext))
            .map(Arc::as_ref)
    }

    // ── Convenience file I/O ─────────────────────────────────────────────────

    /// Loads a scene from the file at `path`, selecting a loader by extension.
    pub fn load_file(&self, path: impl AsRef<Path>) -> Result<Scene> {
        self.load_file_with_options(path, &LoadOptions::default())
    }

    /// Loads a scene from `path` with caller-supplied [`LoadOptions`].
    pub fn load_file_with_options(
        &self,
        path: impl AsRef<Path>,
        options: &LoadOptions,
    ) -> Result<Scene> {
        let path = path.as_ref();
        let ext  = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SolidError::UnsupportedFormat("no file extension".into()))?;

        let loader = self
            .loader_for_extension(ext)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no loader for .{ext}")))?;

        let file   = std::fs::File::open(path).map_err(SolidError::Io)?;
        let mut reader = std::io::BufReader::new(file);
        loader.load(&mut reader, options)
    }

    /// Saves `scene` to the file at `path`, selecting a saver by extension.
    pub fn save_file(&self, scene: &Scene, path: impl AsRef<Path>) -> Result<()> {
        self.save_file_with_options(scene, path, &SaveOptions::default())
    }

    /// Saves `scene` to `path` with caller-supplied [`SaveOptions`].
    pub fn save_file_with_options(
        &self,
        scene: &Scene,
        path: impl AsRef<Path>,
        options: &SaveOptions,
    ) -> Result<()> {
        let path = path.as_ref();
        let ext  = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SolidError::UnsupportedFormat("no file extension".into()))?;

        let saver = self
            .saver_for_extension(ext)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no saver for .{ext}")))?;

        let file   = std::fs::File::create(path).map_err(SolidError::Io)?;
        let mut writer = std::io::BufWriter::new(file);
        saver.save(scene, &mut writer, options)
    }

    /// Loads a scene from an already-open reader using the loader for `format_id`.
    pub fn load_from<R: ReadSeek>(
        &self,
        mut reader: R,
        format_id: &str,
        options: &LoadOptions,
    ) -> Result<Scene> {
        let loader = self
            .loader_by_id(format_id)
            .ok_or_else(|| SolidError::UnsupportedFormat(format!("no loader for '{format_id}'")))?;
        loader.load(&mut reader, options)
    }

    // ── Introspection ────────────────────────────────────────────────────────

    /// Returns an iterator over the [`FormatInfo`] of every registered loader.
    pub fn loader_infos(&self) -> impl Iterator<Item = &FormatInfo> {
        self.loaders.iter().map(|l| l.format_info())
    }

    /// Returns an iterator over the [`FormatInfo`] of every registered saver.
    pub fn saver_infos(&self) -> impl Iterator<Item = &FormatInfo> {
        self.savers.iter().map(|s| s.format_info())
    }

    /// Returns `true` if at least one loader is registered for `ext`.
    pub fn can_load_extension(&self, ext: &str) -> bool {
        self.loader_for_extension(ext).is_some()
    }

    /// Returns `true` if at least one saver is registered for `ext`.
    pub fn can_save_extension(&self, ext: &str) -> bool {
        self.saver_for_extension(ext).is_some()
    }
}
