//! Scene node: a named transform in the scene graph.

use crate::extensions::Extensions;
use crate::geometry::Transform;

/// A stable, opaque identifier for a [`Node`] within a [`Scene`](crate::scene::Scene).
///
/// IDs are assigned by [`SceneBuilder`](crate::builder::SceneBuilder) and
/// remain valid for the lifetime of the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NodeId(pub u32);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node({})", self.0)
    }
}

/// A node in the scene graph, analogous to a transform hierarchy entry in
/// DCC tools and game engines.
///
/// Nodes form a directed acyclic graph (DAG); each node stores its children
/// by [`NodeId`] and a local [`Transform`] relative to its parent.
///
/// Scene-level data (meshes, cameras, lights, skins) are stored in flat
/// arrays on the [`Scene`](crate::scene::Scene) and referenced by index.
#[derive(Debug, Clone)]
pub struct Node {
    /// Unique identifier within the owning scene.
    pub id: NodeId,

    /// Human-readable name (not necessarily unique).
    pub name: String,

    /// Local transform relative to the parent node.
    pub transform: Transform,

    /// Ordered list of child node IDs.
    pub children: Vec<NodeId>,

    /// Parent node ID, or `None` for root nodes.
    pub parent: Option<NodeId>,

    /// Optional index into [`Scene::meshes`](crate::scene::Scene::meshes).
    pub mesh: Option<usize>,

    /// Optional index into [`Scene::cameras`](crate::scene::Scene::cameras).
    pub camera: Option<usize>,

    /// Optional index into [`Scene::lights`](crate::scene::Scene::lights).
    pub light: Option<usize>,

    /// Optional index into [`Scene::skins`](crate::scene::Scene::skins).
    pub skin: Option<usize>,

    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Node {
    /// Creates a new node with the given ID and name, identity transform,
    /// no children, and no attached scene objects.
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            transform: Transform::IDENTITY,
            children: Vec::new(),
            parent: None,
            mesh: None,
            camera: None,
            light: None,
            skin: None,
            extensions: Extensions::new(),
        }
    }

    /// Returns `true` if this node has no child nodes.
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns `true` if this node has at least one attached scene object
    /// (mesh, camera, light, or skin).
    #[inline]
    pub fn has_attachment(&self) -> bool {
        self.mesh.is_some() || self.camera.is_some() || self.light.is_some() || self.skin.is_some()
    }
}
