//! Convenient re-exports for the most commonly used types.
//!
//! Import with `use solid_rs::prelude::*` to bring everything needed for
//! day-to-day scene construction and format-crate implementation into scope.

pub use crate::builder::SceneBuilder;
pub use crate::error::{Result, SolidError};
pub use crate::extensions::Extensions;
pub use crate::geometry::{
    Aabb, Primitive, SkinWeights, Topology, Transform, Vertex,
    MAX_COLOR_CHANNELS, MAX_UV_CHANNELS,
};
pub use crate::registry::Registry;
pub use crate::scene::{
    AlphaMode, Animation, AnimationChannel, AnimationTarget, AreaLight, Camera,
    DirectionalLight, FilterMode, Image, ImageSource, Interpolation, Light,
    LightBase, Material, Mesh, MorphTarget, Node, NodeId, OrthographicCamera,
    PerspectiveCamera, PointLight, Projection, Sampler, Scene, Skin, SpotLight,
    Texture, TextureRef, TextureTransform, WrapMode,
};
pub use crate::traits::{
    FormatInfo, LoadOptions, Loader, ReadSeek, SaveOptions, Saver, SceneVisitor,
};
pub use crate::value::Value;

/// Re-export of `glam` for use in code that constructs geometry.
pub use glam;
