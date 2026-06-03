//! Animation clips and keyframe channels.

use crate::extensions::Extensions;
use crate::scene::node::NodeId;

/// Keyframe interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Interpolation {
    /// Linear interpolation between consecutive keyframes.
    #[default]
    Linear,
    /// Immediate jump to the next keyframe value — no interpolation.
    Step,
    /// Cubic spline (Hermite) interpolation.
    /// `values` stores `[in_tangent, value, out_tangent]` triples per keyframe.
    CubicSpline,
}

/// The scene-graph property that an [`AnimationChannel`] animates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnimationTarget {
    /// Animates [`Node::transform.translation`](crate::scene::Node::transform).
    Translation(NodeId),
    /// Animates [`Node::transform.rotation`](crate::scene::Node::transform).
    Rotation(NodeId),
    /// Animates [`Node::transform.scale`](crate::scene::Node::transform).
    Scale(NodeId),
    /// Animates one morph-target weight on the mesh attached to a node.
    MorphWeight {
        node_id: NodeId,
        target_index: usize,
    },
}

/// A single animation channel: one animated property with its keyframe data.
///
/// `times` and `values` are parallel arrays.  The value stride depends on the
/// target type:
///
/// | Target          | Values per keyframe |
/// |:--------------- |:------------------- |
/// | Translation     | 3 (`xyz`)           |
/// | Rotation        | 4 (`xyzw` quat)     |
/// | Scale           | 3 (`xyz`)           |
/// | MorphWeight     | 1                   |
///
/// For [`Interpolation::CubicSpline`] the stride is tripled:
/// `[in_tangent…, value…, out_tangent…]` per keyframe.
#[derive(Debug, Clone)]
pub struct AnimationChannel {
    /// The property and target node being animated.
    pub target: AnimationTarget,
    /// Interpolation mode for this channel.
    pub interpolation: Interpolation,
    /// Keyframe timestamps in seconds (monotonically increasing).
    pub times: Vec<f32>,
    /// Flat buffer of keyframe values (stride depends on target and interpolation).
    pub values: Vec<f32>,
}

impl AnimationChannel {
    /// Returns the number of keyframes in this channel.
    pub fn keyframe_count(&self) -> usize {
        self.times.len()
    }

    /// Returns the duration of the channel (last timestamp), or `0.0`.
    pub fn duration(&self) -> f32 {
        self.times.last().copied().unwrap_or(0.0)
    }
}

/// A named animation clip composed of one or more [`AnimationChannel`]s.
#[derive(Debug, Clone)]
pub struct Animation {
    /// Human-readable name (e.g. `"Walk"`, `"Attack"`).
    pub name: String,
    /// All animated channels belonging to this clip.
    pub channels: Vec<AnimationChannel>,
    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Animation {
    /// Creates an empty animation clip with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            channels: Vec::new(),
            extensions: Extensions::new(),
        }
    }

    /// Returns the total duration of the clip in seconds.
    pub fn duration(&self) -> f32 {
        self.channels
            .iter()
            .map(|c| c.duration())
            .fold(0.0_f32, f32::max)
    }
}
