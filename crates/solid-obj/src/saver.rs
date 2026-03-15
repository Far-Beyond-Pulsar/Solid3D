//! `ObjSaver` — saves a `solid_rs::Scene` as a Wavefront OBJ file.
//!
//! The `.obj` stream written contains geometry, groups, UV and normals.
//! A companion `.mtl` block is appended as a `# MTL BEGIN` / `# MTL END`
//! comment section that downstream tooling can extract, or you can call
//! [`ObjSaver::save_with_mtl`] to write the MTL to a separate writer.

use std::io::Write;

use glam::Vec3;

use solid_rs::prelude::*;
use solid_rs::scene::{AlphaMode, Scene};
use solid_rs::{Result, SolidError};

use crate::OBJ_FORMAT;

/// Saves a `Scene` as Wavefront OBJ.
///
/// Materials are written to the **same writer** as an embedded block
/// delimited by comments.  Call [`ObjSaver::save_mtl`] to write only the
/// MTL content to a separate stream.
pub struct ObjSaver;

impl Saver for ObjSaver {
    fn format_info(&self) -> &FormatInfo {
        &OBJ_FORMAT
    }

    fn save(
        &self,
        scene: &Scene,
        writer: &mut dyn Write,
        options: &SaveOptions,
    ) -> Result<()> {
        write_obj(scene, writer, options)
    }
}

impl ObjSaver {
    /// Write **only** the MTL content to `writer`.  Useful when you need two
    /// separate files: the `.obj` (via `Saver::save`) and the `.mtl`.
    pub fn save_mtl(scene: &Scene, writer: &mut dyn Write) -> Result<()> {
        write_mtl(scene, writer)
    }
}

// ── OBJ writer ────────────────────────────────────────────────────────────────

fn write_obj(scene: &Scene, w: &mut dyn Write, options: &SaveOptions) -> Result<()> {
    let gen = options.generator.as_deref().unwrap_or("solid-obj");
    let copyright = options.copyright.as_deref().unwrap_or("");

    writeln!(w, "# Wavefront OBJ exported by {gen}").map_err(SolidError::Io)?;
    if !copyright.is_empty() {
        writeln!(w, "# {copyright}").map_err(SolidError::Io)?;
    }
    writeln!(w).map_err(SolidError::Io)?;

    // Reference the companion MTL library
    if !scene.materials.is_empty() {
        writeln!(w, "mtllib scene.mtl").map_err(SolidError::Io)?;
        writeln!(w).map_err(SolidError::Io)?;
    }

    // OBJ uses a global vertex pool; we emit all vertices then per-group faces.
    // For simplicity we use per-mesh vertex pools with an offset counter.
    let mut v_offset  = 1usize; // OBJ indices are 1-based
    let mut vt_offset = 1usize;
    let mut vn_offset = 1usize;

    for (mesh_idx, mesh) in scene.meshes.iter().enumerate() {
        if mesh.vertices.is_empty() { continue; }

        // Find the node name for this mesh (first node that references it)
        let group_name = scene.nodes.iter()
            .find(|n| n.mesh == Some(mesh_idx))
            .map(|n| n.name.as_str())
            .unwrap_or(&mesh.name);

        // ── Vertex pool ───────────────────────────────────────────────────────
        for v in &mesh.vertices {
            let p = v.position;
            writeln!(w, "v {:.6} {:.6} {:.6}", p.x, p.y, p.z).map_err(SolidError::Io)?;
        }
        writeln!(w).map_err(SolidError::Io)?;

        let has_uvs = mesh.vertices.iter().any(|v| v.uvs[0].is_some());
        if has_uvs {
            for v in &mesh.vertices {
                let uv = v.uvs[0].unwrap_or_default();
                let vv = if options.flip_uv_v { 1.0 - uv.y } else { uv.y };
                writeln!(w, "vt {:.6} {:.6}", uv.x, vv).map_err(SolidError::Io)?;
            }
            writeln!(w).map_err(SolidError::Io)?;
        }

        let has_normals = mesh.vertices.iter().any(|v| v.normal.is_some());
        if has_normals {
            for v in &mesh.vertices {
                let n = v.normal.unwrap_or(Vec3::Y);
                writeln!(w, "vn {:.6} {:.6} {:.6}", n.x, n.y, n.z).map_err(SolidError::Io)?;
            }
            writeln!(w).map_err(SolidError::Io)?;
        }

        // ── Faces ─────────────────────────────────────────────────────────────
        writeln!(w, "g {group_name}").map_err(SolidError::Io)?;

        let mut last_mat: Option<usize> = None;
        for prim in &mesh.primitives {
            // Switch material if needed
            if prim.material_index != last_mat {
                match prim.material_index {
                    Some(mi) => {
                        let mat_name = &scene.materials[mi].name;
                        writeln!(w, "usemtl {mat_name}").map_err(SolidError::Io)?;
                    }
                    None => {
                        writeln!(w, "usemtl (none)").map_err(SolidError::Io)?;
                    }
                }
                last_mat = prim.material_index;
            }

            // Emit triangles as faces
            for tri in prim.indices.chunks(3) {
                let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                let face = if has_uvs && has_normals {
                    format!(
                        "f {}/{}/{} {}/{}/{} {}/{}/{}",
                        a + v_offset, a + vt_offset, a + vn_offset,
                        b + v_offset, b + vt_offset, b + vn_offset,
                        c + v_offset, c + vt_offset, c + vn_offset,
                    )
                } else if has_uvs {
                    format!(
                        "f {}/{} {}/{} {}/{}",
                        a + v_offset, a + vt_offset,
                        b + v_offset, b + vt_offset,
                        c + v_offset, c + vt_offset,
                    )
                } else if has_normals {
                    format!(
                        "f {}//{} {}//{} {}//{}",
                        a + v_offset, a + vn_offset,
                        b + v_offset, b + vn_offset,
                        c + v_offset, c + vn_offset,
                    )
                } else {
                    format!(
                        "f {} {} {}",
                        a + v_offset, b + v_offset, c + v_offset,
                    )
                };
                writeln!(w, "{face}").map_err(SolidError::Io)?;
            }
        }
        writeln!(w).map_err(SolidError::Io)?;

        let n_verts = mesh.vertices.len();
        v_offset  += n_verts;
        vt_offset += n_verts;
        vn_offset += n_verts;
    }

    // Append embedded MTL block
    if !scene.materials.is_empty() {
        writeln!(w, "# MTL BEGIN").map_err(SolidError::Io)?;
        write_mtl(scene, w)?;
        writeln!(w, "# MTL END").map_err(SolidError::Io)?;
    }

    Ok(())
}

// ── MTL writer ────────────────────────────────────────────────────────────────

fn write_mtl(scene: &Scene, w: &mut dyn Write) -> Result<()> {
    writeln!(w, "# Wavefront MTL material library").map_err(SolidError::Io)?;
    writeln!(w).map_err(SolidError::Io)?;

    for mat in scene.materials.iter() {
        writeln!(w, "newmtl {}", mat.name).map_err(SolidError::Io)?;

        let c = mat.base_color_factor;
        writeln!(w, "Ka 0.200 0.200 0.200").map_err(SolidError::Io)?;
        writeln!(w, "Kd {:.4} {:.4} {:.4}", c.x, c.y, c.z).map_err(SolidError::Io)?;
        writeln!(w, "Ks 0.000 0.000 0.000").map_err(SolidError::Io)?;

        let e = mat.emissive_factor;
        if e != Vec3::ZERO {
            writeln!(w, "Ke {:.4} {:.4} {:.4}", e.x, e.y, e.z).map_err(SolidError::Io)?;
        }

        // Alpha / dissolve — interpretation depends on alpha mode
        match mat.alpha_mode {
            AlphaMode::Opaque => {} // d 1.0 is the default; omit
            AlphaMode::Mask => {
                writeln!(w, "d {:.4}", 1.0 - mat.alpha_cutoff).map_err(SolidError::Io)?;
            }
            AlphaMode::Blend => {
                let alpha = c.w;
                if alpha < 1.0 {
                    writeln!(w, "d {:.4}", alpha).map_err(SolidError::Io)?;
                }
            }
        }

        // Convert roughness back to Ns (Ns = (1 - roughness)^2 * 1000)
        let ns = (1.0 - mat.roughness_factor).powi(2) * 1000.0;
        writeln!(w, "Ns {:.2}", ns).map_err(SolidError::Io)?;

        // PBR scalars
        if mat.roughness_factor != 1.0 {
            writeln!(w, "Pr {:.4}", mat.roughness_factor).map_err(SolidError::Io)?;
        }
        if mat.metallic_factor != 0.0 {
            writeln!(w, "Pm {:.4}", mat.metallic_factor).map_err(SolidError::Io)?;
        }

        // Texture maps
        if let Some(tr) = &mat.base_color_texture {
            if let Some(uri) = tex_uri(scene, tr.texture_index) {
                writeln!(w, "map_Kd {uri}").map_err(SolidError::Io)?;
            }
        }
        if let Some(tr) = &mat.metallic_roughness_texture {
            if let Some(uri) = tex_uri(scene, tr.texture_index) {
                writeln!(w, "map_Ks {uri}").map_err(SolidError::Io)?;
                writeln!(w, "map_Pr {uri}").map_err(SolidError::Io)?;
            }
        }
        if let Some(tr) = &mat.emissive_texture {
            if let Some(uri) = tex_uri(scene, tr.texture_index) {
                writeln!(w, "map_Ke {uri}").map_err(SolidError::Io)?;
            }
        }
        if let Some(tr) = &mat.normal_texture {
            if let Some(uri) = tex_uri(scene, tr.texture_index) {
                writeln!(w, "map_bump {uri}").map_err(SolidError::Io)?;
                writeln!(w, "norm {uri}").map_err(SolidError::Io)?;
            }
        }

        writeln!(w).map_err(SolidError::Io)?;
    }

    Ok(())
}

fn tex_uri(scene: &Scene, tex_idx: usize) -> Option<&str> {
    let tex = scene.textures.get(tex_idx)?;
    let img = scene.images.get(tex.image_index)?;
    if let solid_rs::scene::ImageSource::Uri(u) = &img.source {
        Some(u.as_str())
    } else {
        None
    }
}
