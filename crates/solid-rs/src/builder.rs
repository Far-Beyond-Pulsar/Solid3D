//! Ergonomic builder for constructing a [`Scene`] incrementally.
//!
//! [`SceneBuilder`] is the primary tool that format-crate loaders use to
//! assemble scenes during parsing — avoiding direct mutation of `Scene`
//! internals.
//!
//! # Example
//!
//! ```rust
//! use solid_rs::builder::SceneBuilder;
//! use solid_rs::scene::{Mesh, Material};
//! use solid_rs::geometry::{Vertex, Primitive};
//! use glam::Vec3;
//!
//! let mut b = SceneBuilder::named("My Scene");
//!
//! let mut mesh = Mesh::new("Cube");
//! mesh.vertices = vec![
//!     Vertex::new(Vec3::new(-1.0, -1.0,  1.0)),
//!     Vertex::new(Vec3::new( 1.0, -1.0,  1.0)),
//!     Vertex::new(Vec3::new( 1.0,  1.0,  1.0)),
//! ];
//! mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
//! let mesh_idx = b.push_mesh(mesh);
//!
//! let root = b.add_root_node("Root");
//! b.attach_mesh(root, mesh_idx);
//!
//! let scene = b.build();
//! assert_eq!(scene.meshes.len(), 1);
//! assert_eq!(scene.nodes.len(), 1);
//! ```

use crate::geometry::Transform;
use crate::scene::{
    Animation, Camera, Image, Light, Material, Mesh, Node, NodeId, Scene, Skin, Texture,
};

/// Incrementally constructs a [`Scene`].
pub struct SceneBuilder {
    scene: Scene,
    next_node_id: u32,
}

impl SceneBuilder {
    /// Creates a builder for an unnamed scene.
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            next_node_id: 0,
        }
    }

    /// Creates a builder whose scene will carry the given name.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            scene: Scene::named(name),
            next_node_id: 0,
        }
    }

    // ── Node management ──────────────────────────────────────────────────────

    fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    /// Adds a root-level node and returns its [`NodeId`].
    pub fn add_root_node(&mut self, name: impl Into<String>) -> NodeId {
        let id = self.alloc_id();
        let node = Node::new(id, name);
        self.scene.nodes.push(node);
        self.scene.roots.push(id);
        id
    }

    /// Adds a child node under `parent` and returns the child's [`NodeId`].
    pub fn add_child_node(&mut self, parent: NodeId, name: impl Into<String>) -> NodeId {
        let id = self.alloc_id();
        let mut node = Node::new(id, name);
        node.parent = Some(parent);
        self.scene.nodes.push(node);
        if let Some(p) = self.scene.node_mut(parent) {
            p.children.push(id);
        }
        id
    }

    /// Sets the local [`Transform`] of a node.
    pub fn set_transform(&mut self, node_id: NodeId, transform: Transform) -> &mut Self {
        if let Some(node) = self.scene.node_mut(node_id) {
            node.transform = transform;
        }
        self
    }

    // ── Data arrays ──────────────────────────────────────────────────────────

    /// Pushes a [`Mesh`] and returns its index.
    pub fn push_mesh(&mut self, mesh: Mesh) -> usize {
        let idx = self.scene.meshes.len();
        self.scene.meshes.push(mesh);
        idx
    }

    /// Pushes a [`Material`] and returns its index.
    pub fn push_material(&mut self, material: Material) -> usize {
        let idx = self.scene.materials.len();
        self.scene.materials.push(material);
        idx
    }

    /// Pushes a [`Texture`] and returns its index.
    pub fn push_texture(&mut self, texture: Texture) -> usize {
        let idx = self.scene.textures.len();
        self.scene.textures.push(texture);
        idx
    }

    /// Pushes an [`Image`] and returns its index.
    pub fn push_image(&mut self, image: Image) -> usize {
        let idx = self.scene.images.len();
        self.scene.images.push(image);
        idx
    }

    /// Pushes a [`Camera`] and returns its index.
    pub fn push_camera(&mut self, camera: Camera) -> usize {
        let idx = self.scene.cameras.len();
        self.scene.cameras.push(camera);
        idx
    }

    /// Pushes a [`Light`] and returns its index.
    pub fn push_light(&mut self, light: Light) -> usize {
        let idx = self.scene.lights.len();
        self.scene.lights.push(light);
        idx
    }

    /// Pushes a [`Skin`] and returns its index.
    pub fn push_skin(&mut self, skin: Skin) -> usize {
        let idx = self.scene.skins.len();
        self.scene.skins.push(skin);
        idx
    }

    /// Pushes an [`Animation`] and returns its index.
    pub fn push_animation(&mut self, animation: Animation) -> usize {
        let idx = self.scene.animations.len();
        self.scene.animations.push(animation);
        idx
    }

    // ── Node attachment ──────────────────────────────────────────────────────

    /// Attaches a mesh index to a node.
    pub fn attach_mesh(&mut self, node_id: NodeId, mesh_index: usize) -> &mut Self {
        if let Some(node) = self.scene.node_mut(node_id) {
            node.mesh = Some(mesh_index);
        }
        self
    }

    /// Attaches a camera index to a node.
    pub fn attach_camera(&mut self, node_id: NodeId, camera_index: usize) -> &mut Self {
        if let Some(node) = self.scene.node_mut(node_id) {
            node.camera = Some(camera_index);
        }
        self
    }

    /// Attaches a light index to a node.
    pub fn attach_light(&mut self, node_id: NodeId, light_index: usize) -> &mut Self {
        if let Some(node) = self.scene.node_mut(node_id) {
            node.light = Some(light_index);
        }
        self
    }

    /// Attaches a skin index to a node.
    pub fn attach_skin(&mut self, node_id: NodeId, skin_index: usize) -> &mut Self {
        if let Some(node) = self.scene.node_mut(node_id) {
            node.skin = Some(skin_index);
        }
        self
    }

    // ── Build ────────────────────────────────────────────────────────────────

    /// Consumes the builder and returns the assembled [`Scene`].
    pub fn build(self) -> Scene {
        self.scene
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}
