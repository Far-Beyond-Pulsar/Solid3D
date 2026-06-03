//! Mesh geometry: vertex buffer, indexed primitives, and morph targets.

use glam::Vec3;

use crate::extensions::Extensions;
use crate::geometry::{Aabb, Primitive, Vertex};

/// A shape key (blend shape / morph target) that offsets vertex attributes.
///
/// Position, normal, and tangent deltas are all optional; loaders only
/// populate the channels present in the source file.
#[derive(Debug, Clone, Default)]
pub struct MorphTarget {
    /// Human-readable name for this morph target (e.g. `"smile"`, `"browUp"`).
    pub name: String,
    /// Per-vertex position deltas (same length as the parent mesh's vertex buffer).
    pub position_deltas: Vec<Vec3>,
    /// Per-vertex normal deltas.
    pub normal_deltas: Vec<Vec3>,
    /// Per-vertex tangent direction deltas (xyz only; w is ignored).
    pub tangent_deltas: Vec<Vec3>,
}

/// Geometric data for a single named mesh object.
///
/// A mesh owns a flat vertex buffer ([`vertices`]) shared across all draw
/// calls ([`primitives`]), optional morph targets, and a cached bounding box.
///
/// # Example
///
/// ```rust
/// use solid_rs::scene::Mesh;
/// use solid_rs::geometry::{Vertex, Primitive};
/// use glam::Vec3;
///
/// let mut mesh = Mesh::new("Triangle");
/// mesh.vertices = vec![
///     Vertex::new(Vec3::new( 0.0,  1.0, 0.0)),
///     Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
///     Vertex::new(Vec3::new( 1.0, -1.0, 0.0)),
/// ];
/// mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
/// ```
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Human-readable name (not necessarily unique).
    pub name: String,

    /// Interleaved vertex buffer shared by all primitives.
    pub vertices: Vec<Vertex>,

    /// One or more indexed draw calls; each has its own index list and
    /// optional material assignment.
    pub primitives: Vec<Primitive>,

    /// Blend shapes / shape keys for morph-target animation.
    pub morph_targets: Vec<MorphTarget>,

    /// Initial blend weights, one per morph target (same length as `morph_targets`, or empty).
    pub morph_weights: Vec<f32>,

    /// Cached axis-aligned bounding box. `None` until [`compute_bounds`] is
    /// called or a loader computes it from the source file.
    pub bounds: Option<Aabb>,

    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Mesh {
    /// Creates an empty mesh with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vertices: Vec::new(),
            primitives: Vec::new(),
            morph_targets: Vec::new(),
            morph_weights: Vec::new(),
            bounds: None,
            extensions: Extensions::new(),
        }
    }

    /// Computes (or recomputes) [`bounds`](Mesh::bounds) from vertex positions.
    pub fn compute_bounds(&mut self) {
        self.bounds = Aabb::from_points(self.vertices.iter().map(|v| v.position));
    }

    /// Returns the total number of indices across all primitives.
    pub fn total_indices(&self) -> usize {
        self.primitives.iter().map(|p| p.indices.len()).sum()
    }

    /// Returns `true` if the mesh has no vertices.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Returns the number of unique vertices.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}
