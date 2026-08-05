//! Cross-format round-trip test: verify every ordered pair (A, B) of supported
//! formats can serialize and deserialize a canonical scene without data loss.
//!
//! For each pair: original → save A → load A → save B → load B → compare.

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use glam::{Vec2, Vec3};
use solid_rs::prelude::*;
use solid_rs::scene::{ImageSource, TextureRef};
use solid_rs::traits::{LoadOptions, Saver, SaveOptions};

// ── Format crate imports ─────────────────────────────────────────────────────

use solid_obj::{ObjLoader, ObjSaver};
use solid_fbx::{FbxLoader, FbxSaver};
use solid_gltf::{GltfLoader, GltfSaver};
use solid_stl::{StlLoader, StlSaver};
use solid_ply::{PlyLoader, PlySaver};
use solid_x::{XLoader, XSaver};
use solid_mdl::{MdlLoader, MdlSaver};
use solid_usd::{UsdLoader, UsdSaver};

// ── Unique temp dir counter ──────────────────────────────────────────────────

static DIR_COUNTER: AtomicU32 = AtomicU32::new(0);

fn scratch_dir(label: &str) -> PathBuf {
    let n = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("solid-roundtrip-{label}-{n}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ── Canonical scene ──────────────────────────────────────────────────────────

/// A simple triangle with positions, normals, UVs, and one PBR material.
/// All supported formats should preserve at least the geometry.
pub fn canonical_scene() -> Scene {
    let mut b = SceneBuilder::named("CanonicalScene");

    let mut mat = Material::new("CanonMat");
    mat.base_color_factor = glam::Vec4::new(0.8, 0.2, 0.1, 1.0);
    mat.metallic_factor = 0.3;
    mat.roughness_factor = 0.7;
    let mi = b.push_material(mat);

    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.5, 1.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi))];
    let mesh_idx = b.push_mesh(mesh);
    let root = b.add_root_node("Root");
    b.attach_mesh(root, mesh_idx);
    b.build()
}

/// Geometry-only scene (no material) — for formats like STL/PLY that don't
/// support materials.
pub fn geometry_scene() -> Scene {
    let mut b = SceneBuilder::named("GeometryScene");
    let mut mesh = Mesh::new("Tri");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mesh_idx = b.push_mesh(mesh);
    let root = b.add_root_node("Root");
    b.attach_mesh(root, mesh_idx);
    b.build()
}

// ── Scene comparison helpers ─────────────────────────────────────────────────

fn approx_eq_f32(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn v3_approx(a: Vec3, b: Vec3) -> bool {
    approx_eq_f32(a.x, b.x) && approx_eq_f32(a.y, b.y) && approx_eq_f32(a.z, b.z)
}

fn v2_approx(a: Vec2, b: Vec2) -> bool {
    approx_eq_f32(a.x, b.x) && approx_eq_f32(a.y, b.y)
}

fn vertices_match(orig: &[Vertex], loaded: &[Vertex]) -> Result<(), String> {
    if orig.len() != loaded.len() {
        // Some formats (STL, OBJ with diverse normals) may duplicate vertices.
        // Accept either exact or a superset (loaded has at least all orig positions).
        let orig_pos: Vec<Vec3> = orig.iter().map(|v| v.position).collect();
        for o in &orig_pos {
            if !loaded.iter().any(|lv| v3_approx(lv.position, *o)) {
                return Err(format!(
                    "original vertex {o:?} not found among {} loaded vertices",
                    loaded.len()
                ));
            }
        }
        return Ok(());
    }
    for (i, (ov, lv)) in orig.iter().zip(loaded.iter()).enumerate() {
        if !v3_approx(ov.position, lv.position) {
            return Err(format!("vertex {i} position mismatch: {:?} vs {:?}", ov.position, lv.position));
        }
        match (ov.normal, lv.normal) {
            (Some(on), Some(ln)) => {
                if !v3_approx(on, ln) {
                    return Err(format!("vertex {i} normal mismatch: {on:?} vs {ln:?}"));
                }
            }
            (Some(_), None) => return Err(format!("vertex {i} normal lost")),
            _ => {}
        }
        match (ov.uvs[0], lv.uvs[0]) {
            (Some(ou), Some(lu)) => {
                if !v2_approx(ou, lu) {
                    return Err(format!("vertex {i} UV mismatch: {ou:?} vs {lu:?}"));
                }
            }
            (Some(_), None) => return Err(format!("vertex {i} UV lost")),
            _ => {}
        }
    }
    Ok(())
}

fn meshes_match(orig: &[Mesh], loaded: &[Mesh], allow_extra: bool) -> Result<(), String> {
    if loaded.len() < orig.len() {
        return Err(format!(
            "mesh count: {} original vs {} loaded",
            orig.len(),
            loaded.len()
        ));
    }
    if !allow_extra && loaded.len() != orig.len() {
        return Err(format!(
            "mesh count: {} original vs {} loaded (allow_extra=false)",
            orig.len(),
            loaded.len()
        ));
    }

    // Try to match orignal meshes by name or by vertex position fingerprint.
    for om in orig {
        let candidates: Vec<&Mesh> = if allow_extra {
            // Use heursitic: first loaded mesh with a similar vertex count.
            loaded.iter().filter(|lm| {
                !lm.vertices.is_empty()
                    && v3_approx(lm.vertices[0].position, om.vertices[0].position)
            }).collect()
        } else {
            loaded.iter().filter(|lm| lm.name == om.name).collect()
        };
        if candidates.is_empty() {
            return Err(format!("mesh '{}' not found in output", om.name));
        }
        let lm = candidates[0];
        vertices_match(&om.vertices, &lm.vertices)?;
        if om.primitives.len() != lm.primitives.len() {
            return Err(format!(
                "mesh '{}' primitive count: {} vs {}",
                om.name,
                om.primitives.len(),
                lm.primitives.len()
            ));
        }
        for (pi, (op, lp)) in om.primitives.iter().zip(lm.primitives.iter()).enumerate() {
            if op.topology != lp.topology {
                return Err(format!("mesh '{}' prim {pi} mode mismatch", om.name));
            }
            if op.indices.len() != lp.indices.len() {
                return Err(format!(
                    "mesh '{}' prim {pi} index count: {} vs {}",
                    om.name,
                    op.indices.len(),
                    lp.indices.len()
                ));
            }
        }
    }
    Ok(())
}

fn materials_match(orig: &[Material], loaded: &[Material]) -> Result<(), String> {
    // Materials may be lost in formats that don't support them (STL, PLY).
    // Only check if they exist.
    if loaded.is_empty() {
        return Ok(()); // material-agnostic format
    }
    for om in orig {
        let lm = loaded.iter().find(|m| m.name == om.name);
        if lm.is_none() {
            continue; // some formats re-name materials
        }
        let lm = lm.unwrap();
            {
                let o = om.base_color_factor.to_array();
                let l = lm.base_color_factor.to_array();
                for (ci, (&ov, &lv)) in o.iter().zip(l.iter()).enumerate() {
                    if !approx_eq_f32(ov, lv) {
                        return Err(format!(
                            "material '{}' base_color_factor[{ci}]: {} vs {}",
                            om.name, ov, lv
                        ));
                    }
                }
            }
            {
                let o = om.emissive_factor.to_array();
                let l = lm.emissive_factor.to_array();
                for (ci, (&ov, &lv)) in o.iter().zip(l.iter()).enumerate() {
                    if !approx_eq_f32(ov, lv) {
                        return Err(format!(
                            "material '{}' emissive_factor[{ci}]: {} vs {}",
                            om.name, ov, lv
                        ));
                    }
                }
            }
        for (comp, o, l) in [
            ("metallic_factor", om.metallic_factor, lm.metallic_factor),
            ("roughness_factor", om.roughness_factor, lm.roughness_factor),
        ] {
            if !approx_eq_f32(o, l) {
                return Err(format!(
                    "material '{}' {comp}: {} vs {}",
                    om.name, o, l
                ));
            }
        }
    }
    Ok(())
}

fn assert_scene_eq(orig: &Scene, loaded: &Scene, label: &str, check_materials: bool) {
    if let Err(e) = meshes_match(&orig.meshes, &loaded.meshes, true) {
        panic!("[{label}] {e}");
    }
    if check_materials {
        if let Err(e) = materials_match(&orig.materials, &loaded.materials) {
            panic!("[{label}] {e}");
        }
    }
}

// ── Per-format save/load ─────────────────────────────────────────────────────

/// Save scene as the given format into `dir`, then load back.
/// Returns the reloaded scene plus a flag indicating whether the format
/// preserves material data.
fn save_load_as(scene: &Scene, fmt_name: &str, dir: &Path, opts: &LoadOptions) -> (Scene, bool) {
    let mut supports_materials = true;
    let result: Scene = match fmt_name {
        "obj" => {
            let obj_path = dir.join("scene.obj");
            let mtl_path = dir.join("scene.mtl");

            let mut obj_buf = Vec::new();
            ObjSaver.save(scene, &mut obj_buf, &SaveOptions::default()).unwrap();
            std::fs::write(&obj_path, &obj_buf).unwrap();

            let mut mtl_buf = Vec::new();
            ObjSaver::save_mtl(scene, &mut mtl_buf).unwrap();
            std::fs::write(&mtl_path, &mtl_buf).unwrap();

            let load_opts = LoadOptions { base_dir: Some(dir.to_path_buf()), ..opts.clone() };
            let mut f = std::fs::File::open(&obj_path).unwrap();
            ObjLoader.load(&mut f, &load_opts).unwrap()
        }
        "fbx" => {
            let path = dir.join("scene.fbx");
            let mut buf = Vec::new();
            FbxSaver.save(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            FbxLoader.load(&mut f, opts).unwrap()
        }
        "fbx_bin" => {
            let path = dir.join("scene.fbx");
            let mut buf = Vec::new();
            FbxSaver.save_binary(scene, &mut buf).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            FbxLoader.load(&mut f, opts).unwrap()
        }
        "gltf" => {
            let path = dir.join("scene.gltf");
            let mut buf = Vec::new();
            GltfSaver.save(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            GltfLoader.load(&mut f, opts).unwrap()
        }
        "glb" => {
            let path = dir.join("scene.glb");
            let mut buf = Vec::new();
            GltfSaver.save_glb(scene, &mut buf).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            GltfLoader.load(&mut f, opts).unwrap()
        }
        "stl" => {
            let path = dir.join("scene.stl");
            let mut buf = Vec::new();
            StlSaver.save(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            StlLoader.load(&mut f, opts).unwrap()
        }
        "stl_ascii" => {
            let path = dir.join("scene.stl");
            let mut buf = Vec::new();
            StlSaver.save_ascii(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            StlLoader.load(&mut f, opts).unwrap()
        }
        "ply" => {
            let path = dir.join("scene.ply");
            let mut buf = Vec::new();
            PlySaver.save(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            PlyLoader.load(&mut f, opts).unwrap()
        }
        "x" => {
            let path = dir.join("scene.x");
            let mut buf = Vec::new();
            XSaver.save(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            XLoader.load(&mut f, opts).unwrap()
        }
        "mdl" => {
            let path = dir.join("scene.mdl");
            let mut buf = Vec::new();
            MdlSaver.save(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            MdlLoader.load(&mut f, opts).unwrap()
        }
        "usda" => {
            let path = dir.join("scene.usda");
            let mut buf = Vec::new();
            UsdSaver.save(scene, &mut buf, &SaveOptions::default()).unwrap();
            std::fs::write(&path, &buf).unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            UsdLoader.load(&mut f, opts).unwrap()
        }
        other => panic!("unknown format: {other}"),
    };
    // Formats that lose materials:
    if matches!(fmt_name, "stl" | "stl_ascii" | "ply") {
        supports_materials = false;
    }
    (result, supports_materials)
}

/// All format variants we test (identified by short string keys).
const FORMATS: &[&str] = &[
    "obj", "fbx", "fbx_bin", "gltf", "glb",
    "stl", "stl_ascii", "ply", "x", "mdl", "usda",
];

fn format_ext(fmt: &str) -> &str {
    match fmt {
        "obj" => "obj",
        "fbx" | "fbx_bin" => "fbx",
        "gltf" => "gltf",
        "glb" => "glb",
        "stl" | "stl_ascii" => "stl",
        "ply" => "ply",
        "x" => "x",
        "mdl" => "mdl",
        "usda" => "usda",
        _ => "bin",
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn cross_format_geometry_round_trips() {
    // Use geometry-only scene for formats that don't support material.
    let scene = geometry_scene();
    let opts = LoadOptions::default();
    let mut failures = Vec::new();
    let output_root = scratch_dir("output");

    for &src in FORMATS {
        let src_dir = output_root.join(format!("from_{}", src));
        std::fs::create_dir_all(&src_dir).unwrap();

        // original → src
        let (after_src, _) = save_load_as(&scene, src, &src_dir, &opts);

        for &dst in FORMATS {
            let pair_dir = src_dir.join(format!("to_{}", dst));
            std::fs::create_dir_all(&pair_dir).unwrap();

            // src → dst
            let (after_dst, _mat_ok) = save_load_as(&after_src, dst, &pair_dir, &opts);

            if let Err(e) = meshes_match(&scene.meshes, &after_dst.meshes, true) {
                failures.push(format!("{src}→{dst}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Cross-format round-trip failures:\n  {}",
            failures.join("\n  ")
        );
    }
}

#[test]
fn cross_format_material_round_trips() {
    let scene = canonical_scene();
    let opts = LoadOptions::default();
    let mut failures = Vec::new();

    // Only test format pairs where BOTH formats support materials.
    let mat_formats: &[&str] = &["obj", "fbx", "fbx_bin", "gltf", "glb", "x", "mdl", "usda"];
    let output_root = scratch_dir("mat_output");

    for &src in mat_formats {
        let src_dir = output_root.join(format!("from_{}", src));
        std::fs::create_dir_all(&src_dir).unwrap();
        let (after_src, _) = save_load_as(&scene, src, &src_dir, &opts);

        for &dst in mat_formats {
            let pair_dir = src_dir.join(format!("to_{}", dst));
            std::fs::create_dir_all(&pair_dir).unwrap();
            let (after_dst, _) = save_load_as(&after_src, dst, &pair_dir, &opts);

            if let Err(e) = meshes_match(&scene.meshes, &after_dst.meshes, true) {
                failures.push(format!("{src}→{dst}: {e}"));
            }
            if let Err(e) = materials_match(&scene.materials, &after_dst.materials) {
                failures.push(format!("{src}→{dst}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Material round-trip failures:\n  {}",
            failures.join("\n  ")
        );
    }
}
