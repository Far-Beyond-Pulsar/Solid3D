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
    let mut mat_index_map: HashMap<String, usize> = HashMap::new();

    if let Some(mtl) = mtl {
        for (name, raw) in &mtl.materials {
            let idx = push_material(&mut b, raw);
            mat_index_map.insert(name.clone(), idx);
        }
    }

    // When the OBJ file supplies explicit `vn` normals, skip smoothing-group
    // computation (the authored normals take precedence).
    let use_sg = obj.normals.is_empty();

    // ── Step 2: one node + mesh per OBJ group ─────────────────────────────────
    for group in &obj.groups {
        let mut mesh     = Mesh::new(&group.name);
        let mut vertices = Vec::<Vertex>::new();
        // Deduplicate vertices: (pos_idx, uv_key, norm_key, smoothing_group) → buffer index
        // The smoothing-group slot is non-zero only when use_sg is true and the
        // face carries a smoothing group, ensuring vertices in different groups
        // are split at hard edges.
        let mut vert_cache: HashMap<(usize, u32, u32, u32), u32> = HashMap::new();

        // Records every triangle that belongs to a smoothing group > 0.
        // Used for post-processing smooth normals.
        let mut sg_tris: Vec<(u32, u32, u32, u32)> = Vec::new(); // (v0, v1, v2, sg)

        for run in &group.face_runs {
            let mat_idx = mat_index_map.get(&run.material).copied();

            let mut tri_indices: Vec<u32> = Vec::new();

            for face in &run.faces {
                let sg = if use_sg { face.smoothing_group } else { 0 };

                // Fan-triangulate polygon
                let corners = face.refs.len();
                if corners < 3 { continue; }

                let mut face_verts = Vec::with_capacity(corners);
                for &(pi, uvi, ni) in &face.refs {
                    let key = (
                        pi,
                        uvi.map(|v| v as u32 + 1).unwrap_or(0),
                        ni.map(|v| v as u32 + 1).unwrap_or(0),
                        // Split vertices between smoothing groups only when
                        // no explicit normals are present.
                        if use_sg && ni.is_none() { sg } else { 0 },
                    );
                    let idx = *vert_cache.entry(key).or_insert_with(|| {
                        let pos  = obj.positions.get(pi).copied().unwrap_or([0.0; 3]);
                        let uv   = uvi.and_then(|i| obj.uvs.get(i)).copied();
                        let norm = ni.and_then(|i| obj.normals.get(i)).copied();

                        let mut v = Vertex::new(Vec3::from_array(pos));
                        if let Some(n) = norm { v = v.with_normal(Vec3::from_array(n)); }
                        if let Some(t) = uv   { v = v.with_uv(Vec2::new(t[0], t[1])); }

                        let idx = vertices.len() as u32;
                        vertices.push(v);
                        idx
                    });
                    face_verts.push(idx);
                }

                for i in 1..corners - 1 {
                    let v0 = face_verts[0];
                    let v1 = face_verts[i];
                    let v2 = face_verts[i + 1];
                    tri_indices.push(v0);
                    tri_indices.push(v1);
                    tri_indices.push(v2);
                    if use_sg && sg > 0 {
                        sg_tris.push((v0, v1, v2, sg));
                    }
                }
            }

            if !tri_indices.is_empty() {
                mesh.primitives.push(Primitive::triangles(tri_indices, mat_idx));
            }
        }

        // ── Smoothing-group normal computation ────────────────────────────────
        if !sg_tris.is_empty() {
            compute_smooth_normals(&mut vertices, &sg_tris);
        }

        if mesh.primitives.is_empty() { continue; }

        mesh.vertices = vertices;
        let mesh_idx  = b.push_mesh(mesh);
        let node_id   = b.add_root_node(&group.name);
        b.attach_mesh(node_id, mesh_idx);
    }

    b.build()
}

// ── Smoothing-group normals ───────────────────────────────────────────────────

/// For each vertex referenced by `sg_tris`, accumulate area-weighted face
/// normals and assign the normalised result.  Vertices in smoothing group 0
/// (hard edges) are left untouched.
fn compute_smooth_normals(vertices: &mut Vec<Vertex>, sg_tris: &[(u32, u32, u32, u32)]) {
    let n = vertices.len();
    let mut accumulated: Vec<Vec3> = vec![Vec3::ZERO; n];

    for &(v0, v1, v2, _sg) in sg_tris {
        let p0 = vertices[v0 as usize].position;
        let p1 = vertices[v1 as usize].position;
        let p2 = vertices[v2 as usize].position;
        // Area-weighted: cross product is not normalised, so larger faces
        // contribute proportionally more.
        let face_normal = (p1 - p0).cross(p2 - p0);
        accumulated[v0 as usize] += face_normal;
        accumulated[v1 as usize] += face_normal;
        accumulated[v2 as usize] += face_normal;
    }

    for (i, acc) in accumulated.iter().enumerate() {
        let len = acc.length();
        if len > 1e-8 {
            vertices[i].normal = Some(*acc / len);
        }
    }
}

// ── Material conversion ───────────────────────────────────────────────────────

fn push_material(b: &mut SceneBuilder, raw: &MtlMaterial) -> usize {
    let mut mat = Material::new(&raw.name);

    let alpha = raw.dissolve.clamp(0.0, 1.0);
    mat.base_color_factor = Vec4::new(raw.kd[0], raw.kd[1], raw.kd[2], alpha);
    mat.emissive_factor   = Vec3::from_array(raw.ke);

    // Prefer explicit PBR scalars when present; otherwise derive from Ns.
    mat.metallic_factor   = raw.pm.unwrap_or(0.0);
    mat.roughness_factor  = raw.pr.unwrap_or_else(|| {
        // Convert specular exponent to roughness: higher Ns → lower roughness
        (1.0 - (raw.ns / 1000.0).clamp(0.0, 1.0)).sqrt()
    });

    if alpha < 1.0 {
        mat.alpha_mode   = solid_rs::scene::AlphaMode::Blend;
        mat.alpha_cutoff = 0.5;
    }
    mat.double_sided = false;

    // ── Textures ──────────────────────────────────────────────────────────────
    if let Some(path) = &raw.map_kd {
        let tex_idx = push_texture(b, &raw.name, "diffuse", path);
        mat.base_color_texture = Some(TextureRef::new(tex_idx));
    }
    // map_Ks, map_Pr, and map_Pm all map to the metallic-roughness slot;
    // last one specified in the MTL wins.
    if let Some(path) = &raw.map_ks {
        let tex_idx = push_texture(b, &raw.name, "specular", path);
        mat.metallic_roughness_texture = Some(TextureRef::new(tex_idx));
    }
    if let Some(path) = &raw.map_roughness {
        let tex_idx = push_texture(b, &raw.name, "roughness", path);
        mat.metallic_roughness_texture = Some(TextureRef::new(tex_idx));
    }
    if let Some(path) = &raw.map_pm {
        let tex_idx = push_texture(b, &raw.name, "metallic", path);
        mat.metallic_roughness_texture = Some(TextureRef::new(tex_idx));
    }
    if let Some(path) = &raw.map_bump {
        let tex_idx = push_texture(b, &raw.name, "normal", path);
        mat.normal_texture = Some(TextureRef::new(tex_idx));
    }
    if let Some(path) = &raw.map_ke {
        let tex_idx = push_texture(b, &raw.name, "emissive", path);
        mat.emissive_texture = Some(TextureRef::new(tex_idx));
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
