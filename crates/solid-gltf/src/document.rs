//! serde structs for the glTF JSON document object model.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfRoot {
    pub asset: GltfAsset,
    pub scene: Option<usize>,
    pub scenes: Vec<GltfScene>,
    pub nodes: Vec<GltfNode>,
    pub meshes: Vec<GltfMesh>,
    pub materials: Vec<GltfMaterial>,
    pub textures: Vec<GltfTexture>,
    pub images: Vec<GltfImage>,
    pub samplers: Vec<GltfSampler>,
    pub accessors: Vec<GltfAccessor>,
    pub buffer_views: Vec<GltfBufferView>,
    pub buffers: Vec<GltfBuffer>,
    pub cameras: Vec<GltfCamera>,
    pub skins: Vec<GltfSkin>,
    pub animations: Vec<GltfAnimation>,
    pub extensions_used: Vec<String>,
    pub extensions_required: Vec<String>,
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfAsset {
    pub version: String,
    pub generator: Option<String>,
    pub min_version: Option<String>,
    pub copyright: Option<String>,
}

impl Default for GltfAsset {
    fn default() -> Self {
        Self { version: "2.0".into(), generator: None, min_version: None, copyright: None }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfScene {
    pub name: Option<String>,
    pub nodes: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfNode {
    pub name: Option<String>,
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
    pub camera: Option<usize>,
    pub skin: Option<usize>,
    pub translation: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
    pub matrix: Option<[f32; 16]>,
    pub weights: Vec<f32>,
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfMesh {
    pub name: Option<String>,
    pub primitives: Vec<GltfPrimitive>,
    pub weights: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfPrimitive {
    pub attributes: HashMap<String, usize>,
    pub indices: Option<usize>,
    pub material: Option<usize>,
    pub mode: Option<u32>,
    pub targets: Vec<HashMap<String, usize>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfMaterial {
    pub name: Option<String>,
    pub pbr_metallic_roughness: Option<GltfPbr>,
    pub normal_texture: Option<GltfNormalTextureInfo>,
    pub occlusion_texture: Option<GltfOcclusionTextureInfo>,
    pub emissive_texture: Option<GltfTextureInfo>,
    pub emissive_factor: Option<[f32; 3]>,
    pub alpha_mode: Option<String>,
    pub alpha_cutoff: Option<f32>,
    pub double_sided: Option<bool>,
    pub extensions: Option<GltfMaterialExtensions>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct GltfMaterialExtensions {
    #[serde(rename = "KHR_materials_specular")]
    pub khr_materials_specular: Option<GltfMaterialsSpecular>,
    #[serde(rename = "KHR_materials_ior")]
    pub khr_materials_ior: Option<GltfMaterialsIor>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfMaterialsSpecular {
    pub specular_factor: Option<f32>,
    pub specular_color_factor: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfMaterialsIor {
    pub ior: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfPbr {
    pub base_color_factor: Option<[f32; 4]>,
    pub base_color_texture: Option<GltfTextureInfo>,
    pub metallic_factor: Option<f32>,
    pub roughness_factor: Option<f32>,
    pub metallic_roughness_texture: Option<GltfTextureInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfTextureInfo {
    pub index: usize,
    pub tex_coord: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfNormalTextureInfo {
    pub index: usize,
    pub tex_coord: Option<usize>,
    pub scale: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfOcclusionTextureInfo {
    pub index: usize,
    pub tex_coord: Option<usize>,
    pub strength: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfTexture {
    pub name: Option<String>,
    pub source: Option<usize>,
    pub sampler: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfImage {
    pub name: Option<String>,
    pub uri: Option<String>,
    pub mime_type: Option<String>,
    pub buffer_view: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfSampler {
    pub name: Option<String>,
    pub mag_filter: Option<u32>,
    pub min_filter: Option<u32>,
    pub wrap_s: Option<u32>,
    pub wrap_t: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfAccessor {
    pub name: Option<String>,
    pub buffer_view: Option<usize>,
    pub byte_offset: usize,
    pub component_type: u32,
    pub normalized: bool,
    pub count: usize,
    #[serde(rename = "type")]
    pub type_: String,
    pub min: Vec<f64>,
    pub max: Vec<f64>,
    pub sparse: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfBufferView {
    pub name: Option<String>,
    pub buffer: usize,
    pub byte_offset: usize,
    pub byte_length: usize,
    pub byte_stride: Option<usize>,
    pub target: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfBuffer {
    pub name: Option<String>,
    pub uri: Option<String>,
    pub byte_length: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfCamera {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    pub perspective: Option<GltfPerspective>,
    pub orthographic: Option<GltfOrthographic>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfPerspective {
    pub yfov: f32,
    pub znear: f32,
    pub zfar: Option<f32>,
    pub aspect_ratio: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfOrthographic {
    pub xmag: f32,
    pub ymag: f32,
    pub znear: f32,
    pub zfar: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfSkin {
    pub name: Option<String>,
    pub inverse_bind_matrices: Option<usize>,
    pub skeleton: Option<usize>,
    pub joints: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfAnimation {
    pub name: Option<String>,
    pub channels: Vec<GltfAnimationChannel>,
    pub samplers: Vec<GltfAnimationSampler>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfAnimationChannel {
    pub sampler: usize,
    pub target: GltfAnimationTarget,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfAnimationTarget {
    pub node: Option<usize>,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GltfAnimationSampler {
    pub input: usize,
    pub interpolation: Option<String>,
    pub output: usize,
}

pub fn num_components(type_: &str) -> usize {
    match type_ {
        "SCALAR" => 1,
        "VEC2"   => 2,
        "VEC3"   => 3,
        "VEC4"   => 4,
        "MAT2"   => 4,
        "MAT3"   => 9,
        "MAT4"   => 16,
        _        => 1,
    }
}

pub fn component_size(component_type: u32) -> usize {
    match component_type {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        5125 | 5126 => 4,
        _           => 4,
    }
}
