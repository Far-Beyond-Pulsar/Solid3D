//! Light source definitions.

use glam::Vec3;

use crate::extensions::Extensions;

/// Properties common to all light types.
#[derive(Debug, Clone, PartialEq)]
pub struct LightBase {
    /// Human-readable name.
    pub name: String,
    /// Emitted colour in linear RGB.  Does not include intensity.
    pub color: Vec3,
    /// Luminous intensity in candela (cd).
    pub intensity: f32,
}

impl LightBase {
    /// Creates a white, 1 cd light with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: Vec3::ONE,
            intensity: 1.0,
        }
    }
}

/// An infinitely-distant, parallel-ray directional light (e.g. a sun).
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub base: LightBase,
    pub extensions: Extensions,
}

/// An omnidirectional point light that emits equally in all directions.
#[derive(Debug, Clone)]
pub struct PointLight {
    pub base: LightBase,
    /// Maximum range in metres. `None` = infinite (physically based fall-off).
    pub range: Option<f32>,
    pub extensions: Extensions,
}

/// A cone-shaped spot light.
#[derive(Debug, Clone)]
pub struct SpotLight {
    pub base: LightBase,
    /// Maximum range in metres. `None` = infinite.
    pub range: Option<f32>,
    /// Half-angle (radians) of the inner bright cone.
    pub inner_cone_angle: f32,
    /// Half-angle (radians) of the outer falloff cone.
    pub outer_cone_angle: f32,
    pub extensions: Extensions,
}

/// A rectangular area light (not universally supported).
#[derive(Debug, Clone)]
pub struct AreaLight {
    pub base: LightBase,
    /// Width of the emitting rectangle in metres.
    pub width: f32,
    /// Height of the emitting rectangle in metres.
    pub height: f32,
    pub extensions: Extensions,
}

/// All supported light varieties.
#[derive(Debug, Clone)]
pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
    Area(AreaLight),
}

impl Light {
    /// Returns the light's human-readable name.
    pub fn name(&self) -> &str {
        match self {
            Self::Directional(l) => &l.base.name,
            Self::Point(l) => &l.base.name,
            Self::Spot(l) => &l.base.name,
            Self::Area(l) => &l.base.name,
        }
    }

    /// Returns the light's linear-RGB colour.
    pub fn color(&self) -> Vec3 {
        match self {
            Self::Directional(l) => l.base.color,
            Self::Point(l) => l.base.color,
            Self::Spot(l) => l.base.color,
            Self::Area(l) => l.base.color,
        }
    }

    /// Returns the light's intensity in candela.
    pub fn intensity(&self) -> f32 {
        match self {
            Self::Directional(l) => l.base.intensity,
            Self::Point(l) => l.base.intensity,
            Self::Spot(l) => l.base.intensity,
            Self::Area(l) => l.base.intensity,
        }
    }

    /// Returns a mutable reference to the common base properties.
    pub fn base_mut(&mut self) -> &mut LightBase {
        match self {
            Self::Directional(l) => &mut l.base,
            Self::Point(l) => &mut l.base,
            Self::Spot(l) => &mut l.base,
            Self::Area(l) => &mut l.base,
        }
    }
}
