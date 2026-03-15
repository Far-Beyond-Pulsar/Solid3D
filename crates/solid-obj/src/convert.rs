//! Converts an [`ObjData`] + optional [`MtlData`] into a `solid_rs::Scene`.

use std::collections::HashMap;

use glam::{Vec2, Vec3, Vec4};

use solid_rs::builder::SceneBuilder;
use solid_rs::geometry::{Primitive, Vertex};
use solid_rs::scene::{Image, ImageSource, Material, Mesh, Texture, TextureRef};
use solid_rs::scene::Scene;

use crate::parser::{MtlData, MtlMaterial, ObjData};

// ── Entry point ───────────────────────────────────────────────────────────────

/// Convert parsed OBJ + MTL data into a `solid_rs::Scene`.
pub(crate) fn obj_to_scene(obj: &ObjData, mtl: Option<&MtlData>) -> Scene {
    let mut b = SceneBuilder::new();

    // ── Step 1: build materials (keyed by name) ───────────────────────────────
    // Map material name → scene material index
    let mut mat_index_map: HashMap<String, usize> = HashMap::new();

    if let Some(mtl) = mtl {
        for (name, raw) in &mtl.materials {
            let idx = push_material(&mut b, raw);
            mat_index_map.insert(name.clone(), idx);
        }
    }

    // ── Step 2: one node + mesh per OBJ group ─────────────────────────────────
    for group in &obj.groups {
        let mut mesh      = Mesh::new(&group.name);
        let mut vertices  = Vec::<Vertex>::new();
        // Deduplicate vertices: (pos_idx, uv_idx, norm_idx) → vertex buffer index
        let mut vert_cache: HashMap<(usize, u32, u32), u32> = HashMap::new();

        for run in &group.face_runs {
            let mat_idx = mat_index_map.get(&run.material).copied();

            let mut tri_indices: Vec<u32> = Vec::new();

            for face in &run.faces {
                // Fan-triangulate polygon
                let corners = face.refs.len();
                if corners < 3 { continue; }

                let mut face_verts = Vec::with_capacity(corners);
                for &(pi, uvi, ni) in &face.refs {
                    let key = (
                        pi,
                        uvi.map(|v| v as u32 + 1).unwrap_or(0),
                        ni.map(|v| v as u32 + 1).unwrap_or(0),
                    );
                    let idx = *vert_cache.entry(key).or_insert_with(|| {
                        let pos   = obj.positions.get(pi).copied().unwrap_or([0.0; 3]);
                        let uv    = uvi.and_then(|i| obj.uvs.get(i)).copied();
                        let norm  = ni.and_then(|i| obj.normals.get(i)).copied();

                        let mut v = Vertex::new(Vec3::from_array(pos));
                        if let Some(n) = norm { v = v.with_normal(Vec3::from_array(n)); }
                        if let Some(t) = uv   { v = v.with_uv(Vec2::new(t[0], t[1])); }

                        let idx = vertices.len() as u32;
                        vertices.push(v);
                        idx
                    });
                    face_verts.push(idx);
                }

                // Fan triangulation: (0,1,2), (0,2,3), ...
                for i in 1..corners - 1 {
                    tri_indices.push(face_verts[0]);
                    tri_indices.push(face_verts[i]);
                    tri_indices.push(face_verts[i + 1]);
                }
            }

            if !tri_indices.is_empty() {
                mesh.primitives.push(Primitive::triangles(tri_indices, mat_idx));
            }
        }

        if mesh.primitives.is_empty() { continue; }

        mesh.vertices = vertices;
        let mesh_idx  = b.push_mesh(mesh);
        let node_id   = b.add_root_node(&group.name);
        b.attach_mesh(node_id, mesh_idx);
    }

    b.build()
}

// ── Material conversion ───────────────────────────────────────────────────────

fn push_material(b: &mut SceneBuilder, raw: &MtlMaterial) -> usize {
    let mut mat = Material::new(&raw.name);

    let alpha = raw.dissolve.clamp(0.0, 1.0);
    mat.base_color_factor  = Vec4::new(raw.kd[0], raw.kd[1], raw.kd[2], alpha);
    mat.emissive_factor    = Vec3::from_array(raw.ke);
    mat.metallic_factor    = 0.0;
    // Convert specular exponent to roughness: higher Ns → lower roughness
    mat.roughness_factor   = (1.0 - (raw.ns / 1000.0).clamp(0.0, 1.0)).sqrt();

    if alpha < 1.0 {
        mat.alpha_mode  = solid_rs::scene::AlphaMode::Blend;
        mat.alpha_cutoff = 0.5;
    }
    mat.double_sided = false;

    // Textures
    if let Some(path) = &raw.map_kd {
        let tex_idx = push_texture(b, &raw.name, "diffuse", path);
        mat.base_color_texture = Some(TextureRef::new(tex_idx));
    }
    if let Some(path) = &raw.map_ks {
        let tex_idx = push_texture(b, &raw.name, "specular", path);
        mat.metallic_roughness_texture = Some(TextureRef::new(tex_idx));
    }
    if let Some(path) = &raw.map_bump {
        let tex_idx = push_texture(b, &raw.name, "normal", path);
        mat.normal_texture = Some(TextureRef::new(tex_idx));
    }
    if let Some(path) = &raw.map_roughness {
        let tex_idx = push_texture(b, &raw.name, "roughness", path);
        mat.metallic_roughness_texture = Some(TextureRef::new(tex_idx));
    }

    b.push_material(mat)
}

fn push_texture(b: &mut SceneBuilder, mat_name: &str, slot: &str, uri: &str) -> usize {
    let tex_name = format!("{mat_name}_{slot}");
    let img = Image {
        name:       tex_name.clone(),
        source:     ImageSource::Uri(uri.to_owned()),
        extensions: Default::default(),
    };
    let img_idx = b.push_image(img);
    let tex     = Texture::new(&tex_name, img_idx);
    b.push_texture(tex)
}
