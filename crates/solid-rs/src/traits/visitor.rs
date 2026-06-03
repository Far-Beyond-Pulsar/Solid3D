//! [`SceneVisitor`] — a double-dispatch interface for walking a [`Scene`].
//!
//! Savers that want to serialise a scene without cloning it can implement
//! this trait and pass `&mut self` to [`Scene::visit`](crate::scene::Scene::visit).
//!
//! All methods have default no-op implementations so you only need to
//! override the ones you care about.

use crate::error::Result;
use crate::scene::{Animation, Camera, Image, Light, Material, Mesh, Node, Skin, Texture};

/// Visitor called for each object in a [`Scene`](crate::scene::Scene).
///
/// # Example
///
/// ```rust,no_run
/// use solid_rs::traits::SceneVisitor;
/// use solid_rs::scene::Mesh;
/// use solid_rs::error::Result;
///
/// struct MeshCounter(usize);
///
/// impl SceneVisitor for MeshCounter {
///     fn visit_mesh(&mut self, mesh: &Mesh, _index: usize) -> Result<()> {
///         self.0 += 1;
///         Ok(())
///     }
/// }
///
/// // scene.visit(&mut MeshCounter(0)).unwrap();
/// ```
pub trait SceneVisitor {
    /// Called once for every node in the scene.
    fn visit_node(&mut self, node: &Node) -> Result<()> {
        let _ = node;
        Ok(())
    }

    /// Called once for each mesh, with its index in `Scene::meshes`.
    fn visit_mesh(&mut self, mesh: &Mesh, index: usize) -> Result<()> {
        let _ = (mesh, index);
        Ok(())
    }

    /// Called once for each material, with its index in `Scene::materials`.
    fn visit_material(&mut self, material: &Material, index: usize) -> Result<()> {
        let _ = (material, index);
        Ok(())
    }

    /// Called once for each texture, with its index in `Scene::textures`.
    fn visit_texture(&mut self, texture: &Texture, index: usize) -> Result<()> {
        let _ = (texture, index);
        Ok(())
    }

    /// Called once for each image, with its index in `Scene::images`.
    fn visit_image(&mut self, image: &Image, index: usize) -> Result<()> {
        let _ = (image, index);
        Ok(())
    }

    /// Called once for each camera, with its index in `Scene::cameras`.
    fn visit_camera(&mut self, camera: &Camera, index: usize) -> Result<()> {
        let _ = (camera, index);
        Ok(())
    }

    /// Called once for each light, with its index in `Scene::lights`.
    fn visit_light(&mut self, light: &Light, index: usize) -> Result<()> {
        let _ = (light, index);
        Ok(())
    }

    /// Called once for each skin, with its index in `Scene::skins`.
    fn visit_skin(&mut self, skin: &Skin, index: usize) -> Result<()> {
        let _ = (skin, index);
        Ok(())
    }

    /// Called once for each animation, with its index in `Scene::animations`.
    fn visit_animation(&mut self, animation: &Animation, index: usize) -> Result<()> {
        let _ = (animation, index);
        Ok(())
    }
}
