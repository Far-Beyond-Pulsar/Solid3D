//! Textures, images, and sampler state.

use crate::extensions::Extensions;

/// Where the source image data lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSource {
    /// Relative or absolute file URI (e.g. `"textures/albedo.png"`).
    Uri(String),
    /// Raw bytes embedded in the file, together with their MIME type
    /// (e.g. `"image/png"` or `"image/jpeg"`).
    Embedded { mime_type: String, data: Vec<u8> },
}

/// A 2-D source image.
///
/// Images are referenced by [`Texture`] objects and are stored separately so
/// the same image can be shared by multiple textures with different samplers.
#[derive(Debug, Clone)]
pub struct Image {
    /// Human-readable name.
    pub name: String,
    /// Location of the image data.
    pub source: ImageSource,
    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Image {
    /// Creates an image that references an external URI.
    pub fn from_uri(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: ImageSource::Uri(uri.into()),
            extensions: Extensions::new(),
        }
    }

    /// Creates an image backed by embedded bytes.
    pub fn embedded(name: impl Into<String>, mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            source: ImageSource::Embedded {
                mime_type: mime_type.into(),
                data,
            },
            extensions: Extensions::new(),
        }
    }
}

// ── Sampler ──────────────────────────────────────────────────────────────────

/// Texture coordinate wrapping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WrapMode {
    /// UV wraps around: `1.1` → `0.1`.
    #[default]
    Repeat,
    /// UV mirrors at every integer boundary.
    MirroredRepeat,
    /// UV is clamped to `[0, 1]`.
    ClampToEdge,
}

/// Texture filtering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterMode {
    /// Nearest-neighbour (point) sampling — no interpolation.
    Nearest,
    /// Bilinear interpolation.
    Linear,
    /// Nearest-neighbour texture, nearest mipmap level.
    NearestMipmapNearest,
    /// Bilinear within the nearest mipmap level.
    LinearMipmapNearest,
    /// Nearest-neighbour texture, linearly blended between mipmap levels.
    NearestMipmapLinear,
    /// Trilinear filtering: bilinear + linear mipmap blend.
    LinearMipmapLinear,
}

impl Default for FilterMode {
    fn default() -> Self {
        Self::LinearMipmapLinear
    }
}

/// Sampler settings that control how a texture is sampled.
#[derive(Debug, Clone, Default)]
pub struct Sampler {
    /// Magnification filter (when the texture is displayed larger than its
    /// native resolution).
    pub mag_filter: FilterMode,
    /// Minification filter (when the texture is displayed smaller).
    pub min_filter: FilterMode,
    /// Horizontal (U / S) wrap mode.
    pub wrap_s: WrapMode,
    /// Vertical (V / T) wrap mode.
    pub wrap_t: WrapMode,
}

// ── Texture ──────────────────────────────────────────────────────────────────

/// A texture: an [`Image`] combined with a [`Sampler`].
///
/// Stored in [`Scene::textures`](crate::scene::Scene::textures).  Materials
/// reference textures by index via [`TextureRef`](crate::scene::TextureRef).
#[derive(Debug, Clone)]
pub struct Texture {
    /// Human-readable name.
    pub name: String,
    /// Index into [`Scene::images`](crate::scene::Scene::images).
    pub image_index: usize,
    /// Sampler settings.
    pub sampler: Sampler,
    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Texture {
    /// Creates a texture pointing at `image_index` with default sampler settings.
    pub fn new(name: impl Into<String>, image_index: usize) -> Self {
        Self {
            name: name.into(),
            image_index,
            sampler: Sampler::default(),
            extensions: Extensions::new(),
        }
    }
}
