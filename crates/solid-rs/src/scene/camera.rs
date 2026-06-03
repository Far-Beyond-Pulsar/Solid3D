//! Camera definitions: perspective and orthographic projections.

use crate::extensions::Extensions;

/// Perspective (frustum) projection settings.
#[derive(Debug, Clone, PartialEq)]
pub struct PerspectiveCamera {
    /// Vertical field-of-view in radians.
    pub fov_y: f32,
    /// Viewport aspect ratio (width / height).
    /// `None` means "inherit from the current viewport".
    pub aspect_ratio: Option<f32>,
    /// Distance to the near clipping plane (must be > 0).
    pub z_near: f32,
    /// Distance to the far clipping plane.
    /// `None` encodes an infinite projection (reversed-Z friendly).
    pub z_far: Option<f32>,
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            fov_y: std::f32::consts::FRAC_PI_4, // 45°
            aspect_ratio: None,
            z_near: 0.01,
            z_far: None,
        }
    }
}

/// Orthographic projection settings.
#[derive(Debug, Clone, PartialEq)]
pub struct OrthographicCamera {
    /// Half-width of the orthographic view volume.
    pub x_mag: f32,
    /// Half-height of the orthographic view volume.
    pub y_mag: f32,
    /// Distance to the near clipping plane.
    pub z_near: f32,
    /// Distance to the far clipping plane.
    pub z_far: f32,
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        Self {
            x_mag: 1.0,
            y_mag: 1.0,
            z_near: 0.01,
            z_far: 1000.0,
        }
    }
}

/// Camera projection type.
#[derive(Debug, Clone, PartialEq)]
pub enum Projection {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

impl Default for Projection {
    fn default() -> Self {
        Self::Perspective(PerspectiveCamera::default())
    }
}

/// A camera attached to a scene [`Node`](crate::scene::Node).
///
/// The camera looks down the **−Z** axis in its local coordinate space
/// (following the glTF / OpenGL convention).
#[derive(Debug, Clone)]
pub struct Camera {
    /// Human-readable name.
    pub name: String,
    /// Projection type and parameters.
    pub projection: Projection,
    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Camera {
    /// Creates a perspective camera with default settings.
    pub fn perspective(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            projection: Projection::default(),
            extensions: Extensions::new(),
        }
    }

    /// Creates an orthographic camera with default settings.
    pub fn orthographic(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            projection: Projection::Orthographic(OrthographicCamera::default()),
            extensions: Extensions::new(),
        }
    }

    /// Returns `true` if this is a perspective camera.
    pub fn is_perspective(&self) -> bool {
        matches!(self.projection, Projection::Perspective(_))
    }
}
