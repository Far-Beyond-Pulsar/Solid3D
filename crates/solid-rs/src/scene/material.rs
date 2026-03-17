//! Canonical physically based material model.

use glam::{Vec2, Vec3, Vec4};

use crate::extensions::Extensions;

/// A reference from a material slot to an entry in
/// [`Scene::textures`](crate::scene::Scene::textures).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureRef {
    /// Index into [`Scene::textures`](crate::scene::Scene::textures).
    pub texture_index: usize,
    /// UV channel to sample (0 = primary set).
    pub uv_channel: usize,
    /// Optional affine UV transform applied before sampling.
    pub transform: Option<TextureTransform>,
}

impl TextureRef {
    /// Creates a plain texture reference with no UV transform.
    pub fn new(texture_index: usize) -> Self {
        Self { texture_index, uv_channel: 0, transform: None }
    }
}

/// Affine 2-D transform applied to UV coordinates before sampling.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureTransform {
    /// UV translation offset.
    pub offset: Vec2,
    /// Rotation angle in radians.
    pub rotation: f32,
    /// UV scale factors.
    pub scale: Vec2,
}

impl Default for TextureTransform {
    fn default() -> Self {
        Self { offset: Vec2::ZERO, rotation: 0.0, scale: Vec2::ONE }
    }
}

/// Controls how transparent pixels are handled during rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlphaMode {
    /// All pixels are rendered as fully opaque; the alpha channel is ignored.
    #[default]
    Opaque,

    /// Alpha-test (cut-out transparency): pixels whose alpha is below
    /// [`Material::alpha_cutoff`] are fully discarded.
    Mask,

    /// Standard blended transparency; fragments are sorted and blended.
    Blend,
}

/// A canonical physically based material description.
/// 
/// The schema keeps the widely-used metallic/roughness controls while also
/// carrying explicit dielectric specular and IOR terms. That additive shape is
/// deliberate groundwork for AAA renderers that need a canonical source of
/// truth without collapsing authored workflows during import.
///
/// Format-specific loaders may store additional properties in
/// [`extensions`](Material::extensions).
#[derive(Debug, Clone)]
pub struct Material {
    /// Human-readable name.
    pub name: String,

    // ── Base colour ──────────────────────────────────────────────────────────
    /// Linear RGBA base-colour multiplier. Applied on top of
    /// `base_color_texture` if present.
    pub base_color_factor: Vec4,
    /// Optional base-colour texture.
    pub base_color_texture: Option<TextureRef>,

    // ── Metallic-roughness ───────────────────────────────────────────────────
    /// 0 = dielectric, 1 = metallic.
    pub metallic_factor: f32,
    /// 0 = smooth (mirror), 1 = rough.
    pub roughness_factor: f32,
    /// Optional combined metallic (B) / roughness (G) texture.
    pub metallic_roughness_texture: Option<TextureRef>,

    // ── Explicit dielectric specular / IOR ───────────────────────────────────
    /// Linear RGB specular tint for explicit dielectric workflows.
    ///
    /// A value of white preserves the authored Fresnel colour derived from
    /// [`Material::ior`].
    pub specular_color: Vec3,
    /// Scalar multiplier for explicit specular response.
    pub specular_weight: f32,
    /// Index of refraction for the dielectric interface.
    ///
    /// This stays alongside metallic/roughness so importers do not have to
    /// heuristically collapse one authored workflow into another.
    pub ior: f32,

    // ── Surface detail ───────────────────────────────────────────────────────
    /// Optional tangent-space normal map.
    pub normal_texture: Option<TextureRef>,
    /// Normal-map scale factor.
    pub normal_scale: f32,
    /// Optional ambient occlusion texture (R channel).
    pub occlusion_texture: Option<TextureRef>,
    /// Occlusion strength multiplier.
    pub occlusion_strength: f32,

    // ── Emission ─────────────────────────────────────────────────────────────
    /// Linear RGB emissive colour multiplier.
    pub emissive_factor: Vec3,
    /// Optional emissive texture.
    pub emissive_texture: Option<TextureRef>,

    // ── Alpha ────────────────────────────────────────────────────────────────
    /// Alpha blending strategy.
    pub alpha_mode: AlphaMode,
    /// Alpha threshold for [`AlphaMode::Mask`].
    pub alpha_cutoff: f32,

    /// Whether back faces should be rendered.
    pub double_sided: bool,

    /// Format-specific extension data.
    pub extensions: Extensions,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: String::new(),
            base_color_factor: Vec4::ONE,
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
            specular_color: Vec3::ONE,
            specular_weight: 1.0,
            ior: 1.5,
            normal_texture: None,
            normal_scale: 1.0,
            occlusion_texture: None,
            occlusion_strength: 1.0,
            emissive_factor: Vec3::ZERO,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
            extensions: Extensions::new(),
        }
    }
}

impl Material {
    /// Creates a default material with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }

    /// Creates a simple untextured material with the given base colour.
    pub fn solid_color(name: impl Into<String>, color: Vec4) -> Self {
        Self { name: name.into(), base_color_factor: color, ..Default::default() }
    }
}
