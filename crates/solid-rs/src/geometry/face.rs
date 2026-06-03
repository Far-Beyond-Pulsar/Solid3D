//! Primitive topology and indexed draw calls.
//!
//! A [`Primitive`] is one draw call within a [`Mesh`](crate::scene::Mesh):
//! it pairs a [`Topology`] with a list of vertex indices and an optional
//! material assignment.  A mesh may contain multiple primitives — for example
//! one per material group in an OBJ file.

/// How a sequence of vertex indices should be interpreted to form geometric
/// primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Topology {
    /// Each consecutive group of three indices forms an independent triangle.
    #[default]
    TriangleList,

    /// The first three indices form the first triangle; each subsequent index
    /// adds a new triangle sharing the previous two vertices.
    TriangleStrip,

    /// Each consecutive pair of indices forms an independent line segment.
    LineList,

    /// Indices form a connected polyline.
    LineStrip,

    /// Each index represents an individual point.
    PointList,

    /// Each consecutive group of four indices forms a quad face.
    /// Loaders that encounter quads in a format that only supports triangles
    /// should triangulate them and emit [`TriangleList`](Topology::TriangleList).
    QuadList,

    /// Arbitrary n-gon polygon.  The `indices` field is a flat list; the
    /// accompanying loop-count information must be stored in a format-specific
    /// extension.
    Polygon,
}

impl Topology {
    /// Returns a human-readable name for this topology.
    pub fn name(self) -> &'static str {
        match self {
            Self::TriangleList => "TriangleList",
            Self::TriangleStrip => "TriangleStrip",
            Self::LineList => "LineList",
            Self::LineStrip => "LineStrip",
            Self::PointList => "PointList",
            Self::QuadList => "QuadList",
            Self::Polygon => "Polygon",
        }
    }
}

/// A single indexed draw call within a [`Mesh`](crate::scene::Mesh).
///
/// Indices reference vertices in the parent mesh's vertex buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct Primitive {
    /// How indices are interpreted.
    pub topology: Topology,

    /// Indices into the parent mesh's [`vertices`](crate::scene::Mesh::vertices) buffer.
    pub indices: Vec<u32>,

    /// Optional index into [`Scene::materials`](crate::scene::Scene::materials).
    /// `None` means use the default / no material.
    pub material_index: Option<usize>,
}

impl Primitive {
    /// Convenience constructor for a [`TriangleList`](Topology::TriangleList) primitive.
    pub fn triangles(indices: Vec<u32>, material_index: Option<usize>) -> Self {
        Self {
            topology: Topology::TriangleList,
            indices,
            material_index,
        }
    }

    /// Convenience constructor for a [`LineList`](Topology::LineList) primitive.
    pub fn lines(indices: Vec<u32>, material_index: Option<usize>) -> Self {
        Self {
            topology: Topology::LineList,
            indices,
            material_index,
        }
    }

    /// Convenience constructor for a [`PointList`](Topology::PointList) primitive.
    pub fn points(indices: Vec<u32>, material_index: Option<usize>) -> Self {
        Self {
            topology: Topology::PointList,
            indices,
            material_index,
        }
    }

    /// Returns the number of complete geometric elements (triangles, lines,
    /// points, or quads) in this primitive.
    pub fn element_count(&self) -> usize {
        match self.topology {
            Topology::TriangleList => self.indices.len() / 3,
            Topology::TriangleStrip => self.indices.len().saturating_sub(2),
            Topology::LineList => self.indices.len() / 2,
            Topology::LineStrip => self.indices.len().saturating_sub(1),
            Topology::PointList => self.indices.len(),
            Topology::QuadList => self.indices.len() / 4,
            Topology::Polygon => 1,
        }
    }

    /// Returns `true` if this primitive contains no indices.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}
