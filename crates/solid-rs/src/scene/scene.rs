//! Top-level scene container.

use std::collections::HashMap;

use crate::extensions::Extensions;
use crate::scene::{Animation, Camera, Image, Light, Material, Mesh, Node, NodeId, Skin, Texture};
use crate::value::Value;

/// Free-form metadata stored in the scene header.
///
/// Format crates populate these fields from whatever header information the
/// source file provides.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// The tool or library that created the file (e.g. `"Blender 4.1"`).
    pub generator: Option<String>,
    /// Copyright notice from the file header.
    pub copyright: Option<String>,
    /// Human-readable identifier of the source format (e.g. `"glTF 2.0"`).
    pub source_format: Option<String>,
    /// Arbitrary additional metadata key-value pairs.
    pub extra: HashMap<String, Value>,
}

/// A complete 3-D scene.
///
/// The scene graph is a directed acyclic graph (DAG) rooted at one or more
/// nodes listed in [`roots`](Scene::roots).  All scene objects (meshes,
/// materials, textures, …) are stored in flat [`Vec`]s and referenced by
/// index from nodes and materials.
///
/// # Building a Scene
///
/// Use [`SceneBuilder`](crate::builder::SceneBuilder) rather than mutating
/// `Scene` directly:
///
/// ```rust
/// use solid_rs::builder::SceneBuilder;
/// use solid_rs::scene::Mesh;
///
/// let mut b = SceneBuilder::named("My Scene");
/// let mesh_idx = b.push_mesh(Mesh::new("Cube"));
/// let root     = b.add_root_node("Root");
/// b.attach_mesh(root, mesh_idx);
/// let scene = b.build();
///
/// assert_eq!(scene.name, "My Scene");
/// assert_eq!(scene.meshes.len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Scene {
    /// Scene name (may be empty for unnamed scenes).
    pub name: String,

    /// IDs of the root nodes (top-level nodes with no parent).
    pub roots: Vec<NodeId>,

    /// All nodes in the scene, in insertion order.
    pub nodes: Vec<Node>,

    /// All mesh objects.
    pub meshes: Vec<Mesh>,

    /// All materials referenced by primitives.
    pub materials: Vec<Material>,

    /// All texture objects (image + sampler pairs).
    pub textures: Vec<Texture>,

    /// All source images (file URI or embedded blob).
    pub images: Vec<Image>,

    /// All camera definitions.
    pub cameras: Vec<Camera>,

    /// All light definitions.
    pub lights: Vec<Light>,

    /// All skeletal skin definitions.
    pub skins: Vec<Skin>,

    /// All animation clips.
    pub animations: Vec<Animation>,

    /// Scene-level metadata from the file header.
    pub metadata: Metadata,

    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Scene {
    /// Creates an empty, unnamed scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty scene with the given name.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    // ── Node lookup ──────────────────────────────────────────────────────────

    /// Returns a shared reference to the node with `id`, or `None`.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Returns a mutable reference to the node with `id`, or `None`.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    // ── Statistics ───────────────────────────────────────────────────────────

    /// Total number of vertices across all meshes.
    pub fn total_vertex_count(&self) -> usize {
        self.meshes.iter().map(|m| m.vertices.len()).sum()
    }

    /// Total number of indices across all meshes.
    pub fn total_index_count(&self) -> usize {
        self.meshes.iter().map(|m| m.total_indices()).sum()
    }

    // ── Traversal ────────────────────────────────────────────────────────────

    /// Calls `f` for every node reachable from `root` in depth-first order.
    pub fn walk_from(&self, root: NodeId, f: &mut impl FnMut(&Node)) {
        if let Some(node) = self.node(root) {
            f(node);
            for &child_id in &node.children.clone() {
                self.walk_from(child_id, f);
            }
        }
    }

    /// Calls `f` for every node in the scene in insertion order.
    pub fn walk_all(&self, f: &mut impl FnMut(&Node)) {
        for node in &self.nodes {
            f(node);
        }
    }

    /// Accepts a [`SceneVisitor`](crate::traits::SceneVisitor), calling its
    /// `visit_*` methods for every object in the scene.
    pub fn visit(&self, visitor: &mut dyn crate::traits::SceneVisitor) -> crate::error::Result<()> {
        for node in &self.nodes {
            visitor.visit_node(node)?;
        }
        for (i, mesh) in self.meshes.iter().enumerate() {
            visitor.visit_mesh(mesh, i)?;
        }
        for (i, mat) in self.materials.iter().enumerate() {
            visitor.visit_material(mat, i)?;
        }
        for (i, tex) in self.textures.iter().enumerate() {
            visitor.visit_texture(tex, i)?;
        }
        for (i, img) in self.images.iter().enumerate() {
            visitor.visit_image(img, i)?;
        }
        for (i, cam) in self.cameras.iter().enumerate() {
            visitor.visit_camera(cam, i)?;
        }
        for (i, light) in self.lights.iter().enumerate() {
            visitor.visit_light(light, i)?;
        }
        for (i, skin) in self.skins.iter().enumerate() {
            visitor.visit_skin(skin, i)?;
        }
        for (i, anim) in self.animations.iter().enumerate() {
            visitor.visit_animation(anim, i)?;
        }
        Ok(())
    }
}
