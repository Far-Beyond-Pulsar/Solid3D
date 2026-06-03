//! Vertex types and per-vertex attribute data.
//!
//! A [`Vertex`] represents a single point in a mesh's vertex buffer.  It
//! carries a mandatory position plus a rich set of optional attributes:
//! surface normals, tangents, up to [`MAX_UV_CHANNELS`] texture coordinate
//! channels, up to [`MAX_COLOR_CHANNELS`] colour channels, and skeletal
//! skinning weights.

use glam::{Vec2, Vec3, Vec4};

/// Maximum number of UV (texture coordinate) channels stored per vertex.
pub const MAX_UV_CHANNELS: usize = 8;

/// Maximum number of vertex colour channels stored per vertex.
pub const MAX_COLOR_CHANNELS: usize = 4;

/// Per-vertex skeletal animation influences.
///
/// Each vertex may be affected by up to four joints simultaneously.
/// Weights should be normalised (sum to 1.0).
#[derive(Debug, Clone, PartialEq)]
pub struct SkinWeights {
    /// Indices into the owning [`Skin`](crate::scene::Skin)'s joint list.
    pub joints: [u16; 4],
    /// Blend weights — should sum to `1.0`.
    pub weights: [f32; 4],
}

impl Default for SkinWeights {
    fn default() -> Self {
        Self {
            joints: [0; 4],
            weights: [0.0; 4],
        }
    }
}

/// A single vertex in a [`Mesh`](crate::scene::Mesh) vertex buffer.
///
/// All attribute fields except `position` are optional; loaders set only
/// the attributes present in the source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    /// Object-space (or world-space) position.
    pub position: Vec3,

    /// Surface normal vector (expected to be unit length).
    pub normal: Option<Vec3>,

    /// Tangent vector for normal-map lighting.
    /// The `w` component encodes the bitangent handedness: `+1.0` or `-1.0`.
    pub tangent: Option<Vec4>,

    /// Per-vertex colour channels (linear RGBA).
    /// Index 0 is the primary colour; higher indices are auxiliary channels.
    pub colors: [Option<Vec4>; MAX_COLOR_CHANNELS],

    /// Texture coordinate channels.
    /// Index 0 is the primary UV set; higher indices are lightmap UVs, etc.
    pub uvs: [Option<Vec2>; MAX_UV_CHANNELS],

    /// Skeletal skinning weights, if this mesh is skinned.
    pub skin_weights: Option<SkinWeights>,
}

impl Vertex {
    /// Creates a vertex with only `position` set; all other fields are `None`.
    #[inline]
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            normal: None,
            tangent: None,
            colors: [None; MAX_COLOR_CHANNELS],
            uvs: [None; MAX_UV_CHANNELS],
            skin_weights: None,
        }
    }

    /// Returns the primary (channel 0) UV coordinate.
    #[inline]
    pub fn uv(&self) -> Option<Vec2> {
        self.uvs[0]
    }

    /// Returns the primary (channel 0) vertex colour.
    #[inline]
    pub fn color(&self) -> Option<Vec4> {
        self.colors[0]
    }

    /// Builder-style setter for the surface normal.
    #[inline]
    pub fn with_normal(mut self, n: Vec3) -> Self {
        self.normal = Some(n);
        self
    }

    /// Builder-style setter for the primary UV coordinate.
    #[inline]
    pub fn with_uv(mut self, uv: Vec2) -> Self {
        self.uvs[0] = Some(uv);
        self
    }

    /// Builder-style setter for the primary vertex colour.
    #[inline]
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.colors[0] = Some(color);
        self
    }

    /// Builder-style setter for skinning weights.
    #[inline]
    pub fn with_skin_weights(mut self, w: SkinWeights) -> Self {
        self.skin_weights = Some(w);
        self
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self::new(Vec3::ZERO)
    }
}
