mod common;
use common::*;
use solid_obj::ObjLoader;
use solid_rs::prelude::*;
use std::io::Cursor;

// ── Empty / trivial ───────────────────────────────────────────────────────────

#[test]
fn loader_empty_file_gives_empty_scene() {
    let scene = load_obj_str("");
    assert_eq!(scene.meshes.len(), 0, "empty OBJ should produce no meshes");
}

// ── Geometry ─────────────────────────────────────────────────────────────────

const TRIANGLE_OBJ: &str = "\
v 0.0 1.0 0.0
v -1.0 -1.0 0.0
v 1.0 -1.0 0.0
f 1 2 3
";

#[test]
fn loader_single_triangle_vertex_count() {
    let scene = load_obj_str(TRIANGLE_OBJ);
    assert_eq!(scene.meshes.len(), 1);
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_single_triangle_index_count() {
    let scene = load_obj_str(TRIANGLE_OBJ);
    let prim = &scene.meshes[0].primitives[0];
    assert_eq!(prim.indices.len(), 3);
}

#[test]
fn loader_positions_correct() {
    let scene = load_obj_str(TRIANGLE_OBJ);
    let verts = &scene.meshes[0].vertices;
    // Vertices may be reordered; collect positions and check set membership
    let positions: Vec<_> = verts.iter().map(|v| v.position).collect();
    let expected = [
        glam::Vec3::new(0.0, 1.0, 0.0),
        glam::Vec3::new(-1.0, -1.0, 0.0),
        glam::Vec3::new(1.0, -1.0, 0.0),
    ];
    for exp in &expected {
        assert!(
            positions.iter().any(|p| (*p - *exp).length() < 1e-4),
            "expected position {:?} not found",
            exp
        );
    }
}

#[test]
fn loader_normals_parsed() {
    let obj = "\
v 0.0 1.0 0.0
v -1.0 -1.0 0.0
v 1.0 -1.0 0.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
vn 0.0 0.0 1.0
f 1//1 2//2 3//3
";
    let scene = load_obj_str(obj);
    let verts = &scene.meshes[0].vertices;
    assert!(
        verts.iter().all(|v| v.normal.is_some()),
        "every vertex should have a normal when vn lines are present"
    );
    for v in verts {
        let n = v.normal.unwrap();
        assert!(
            (n.z - 1.0).abs() < 1e-4,
            "normal z should be 1.0, got {:?}",
            n
        );
    }
}

#[test]
fn loader_uvs_parsed() {
    let obj = "\
v 0.0 1.0 0.0
v -1.0 -1.0 0.0
v 1.0 -1.0 0.0
vt 0.5 1.0
vt 0.0 0.0
vt 1.0 0.0
f 1/1 2/2 3/3
";
    let scene = load_obj_str(obj);
    let verts = &scene.meshes[0].vertices;
    assert!(
        verts.iter().all(|v| v.uvs[0].is_some()),
        "every vertex should have a UV when vt lines are present"
    );
}

// ── Groups / objects ──────────────────────────────────────────────────────────

#[test]
fn loader_multiple_objects_become_meshes() {
    let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
v 0 0 1
v 1 0 1
v 0 1 1
o ObjectA
f 1 2 3
o ObjectB
f 4 5 6
";
    let scene = load_obj_str(obj);
    assert_eq!(
        scene.meshes.len(),
        2,
        "two 'o' directives should give two meshes"
    );
}

#[test]
fn loader_group_directive_creates_primitive() {
    let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
v 0 0 1
v 1 0 1
v 0 1 1
g GroupA
f 1 2 3
g GroupB
f 4 5 6
";
    let scene = load_obj_str(obj);
    assert_eq!(
        scene.meshes.len(),
        2,
        "'g' directives should create separate meshes"
    );
}

// ── Material references ───────────────────────────────────────────────────────

#[test]
fn loader_usemtl_assigns_material() {
    let obj = "\
mtllib scene.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
";
    let mtl = "\
newmtl Red
Kd 1.0 0.0 0.0
";
    let scene = load_obj_with_mtl(obj, mtl);
    assert!(!scene.materials.is_empty(), "material should be loaded");
    let prim = &scene.meshes[0].primitives[0];
    assert!(
        prim.material_index.is_some(),
        "primitive should reference a material"
    );
}

#[test]
fn loader_mtl_diffuse_color_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nKd 0.6 0.3 0.1\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let mat = &scene.materials[0];
    let c = mat.base_color_factor;
    assert!((c.x - 0.6).abs() < 1e-4, "Kd.r mismatch: {}", c.x);
    assert!((c.y - 0.3).abs() < 1e-4, "Kd.g mismatch: {}", c.y);
    assert!((c.z - 0.1).abs() < 1e-4, "Kd.b mismatch: {}", c.z);
}

#[test]
fn loader_mtl_emissive_color_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nKe 0.1 0.2 0.4\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let e = scene.materials[0].emissive_factor;
    assert!((e.x - 0.1).abs() < 1e-4, "Ke.r mismatch: {}", e.x);
    assert!((e.y - 0.2).abs() < 1e-4, "Ke.g mismatch: {}", e.y);
    assert!((e.z - 0.4).abs() < 1e-4, "Ke.b mismatch: {}", e.z);
}

#[test]
fn loader_mtl_shininess_to_roughness() {
    // Ns 250 → roughness = sqrt(1 - 250/1000) = sqrt(0.75) ≈ 0.866
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nNs 250\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let r = scene.materials[0].roughness_factor;
    let expected = (1.0_f32 - (250.0_f32 / 1000.0)).sqrt();
    assert!(
        (r - expected).abs() < 1e-4,
        "roughness from Ns mismatch: {} vs {}",
        r,
        expected
    );
}

#[test]
fn loader_mtl_opacity_d_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nd 0.5\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let mat = &scene.materials[0];
    assert!(
        (mat.base_color_factor.w - 0.5).abs() < 1e-4,
        "alpha from 'd' mismatch"
    );
    assert_eq!(mat.alpha_mode, solid_rs::scene::AlphaMode::Blend);
}

#[test]
fn loader_mtl_opacity_tr_parsed() {
    // Tr 0.3 → dissolve = 1 - 0.3 = 0.7
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nTr 0.3\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let alpha = scene.materials[0].base_color_factor.w;
    assert!(
        (alpha - 0.7).abs() < 1e-4,
        "alpha from 'Tr' mismatch: {}",
        alpha
    );
}

#[test]
fn loader_mtl_map_kd_texture_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nmap_Kd diffuse.png\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let mat = &scene.materials[0];
    assert!(
        mat.base_color_texture.is_some(),
        "base_color_texture should be present"
    );
    let tex_idx = mat.base_color_texture.as_ref().unwrap().texture_index;
    assert_eq!(image_uri(&scene, tex_idx), Some("diffuse.png"));
}

#[test]
fn loader_mtl_map_bump_normal_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nmap_bump normal.png\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let mat = &scene.materials[0];
    assert!(
        mat.normal_texture.is_some(),
        "normal_texture should be present"
    );
    let tex_idx = mat.normal_texture.as_ref().unwrap().texture_index;
    assert_eq!(image_uri(&scene, tex_idx), Some("normal.png"));
}

#[test]
fn loader_mtl_pr_roughness_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nPr 0.4\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let r = scene.materials[0].roughness_factor;
    assert!((r - 0.4).abs() < 1e-4, "Pr roughness mismatch: {}", r);
}

#[test]
fn loader_mtl_pm_metallic_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nPm 0.6\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let m = scene.materials[0].metallic_factor;
    assert!((m - 0.6).abs() < 1e-4, "Pm metallic mismatch: {}", m);
}

#[test]
fn loader_mtl_map_ke_emissive_texture_parsed() {
    let obj = "mtllib scene.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n";
    let mtl = "newmtl M\nmap_Ke emit.png\n";
    let scene = load_obj_with_mtl(obj, mtl);
    let mat = &scene.materials[0];
    assert!(
        mat.emissive_texture.is_some(),
        "emissive_texture should be present"
    );
    let tex_idx = mat.emissive_texture.as_ref().unwrap().texture_index;
    assert_eq!(image_uri(&scene, tex_idx), Some("emit.png"));
}

// ── Negative indices ──────────────────────────────────────────────────────────

#[test]
fn loader_negative_index_resolved_correctly() {
    // -3 = first vertex, -2 = second, -1 = third when there are 3 positions
    let obj = "\
v 0.0 1.0 0.0
v -1.0 -1.0 0.0
v 1.0 -1.0 0.0
f -3 -2 -1
";
    let scene = load_obj_str(obj);
    assert_eq!(scene.meshes.len(), 1);
    assert_eq!(scene.meshes[0].vertices.len(), 3);
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

// ── N-gon triangulation ───────────────────────────────────────────────────────

#[test]
fn loader_ngon_fan_triangulated() {
    // A quad face should produce 2 triangles (6 indices)
    let obj = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3 4
";
    let scene = load_obj_str(obj);
    let prim = &scene.meshes[0].primitives[0];
    assert_eq!(
        prim.indices.len(),
        6,
        "quad should be fan-triangulated to 6 indices"
    );
}

// ── Smoothing groups ──────────────────────────────────────────────────────────

#[test]
fn loader_smoothing_group_parsed() {
    // Without explicit vn lines, sg > 0 triggers smooth normal computation.
    let obj = "\
v 0.0 1.0 0.0
v -1.0 -1.0 0.0
v 1.0 -1.0 0.0
s 1
f 1 2 3
";
    let scene = load_obj_str(obj);
    let verts = &scene.meshes[0].vertices;
    assert!(
        verts.iter().all(|v| v.normal.is_some()),
        "smoothing group 1 should trigger normal computation"
    );
}

#[test]
fn loader_smoothing_group_off() {
    // s off → smoothing_group = 0, no normals computed (and no explicit vn).
    let obj = "\
v 0.0 1.0 0.0
v -1.0 -1.0 0.0
v 1.0 -1.0 0.0
s off
f 1 2 3
";
    let scene = load_obj_str(obj);
    let verts = &scene.meshes[0].vertices;
    assert!(
        verts.iter().all(|v| v.normal.is_none()),
        "'s off' should leave vertex normals unset"
    );
}

// ── Error handling ────────────────────────────────────────────────────────────

#[test]
fn loader_rejects_completely_invalid() {
    // Invalid UTF-8 bytes should cause read_to_string to fail.
    let invalid: Vec<u8> = vec![0xFF, 0xFE, 0x80, 0x81, 0x00];
    let mut cursor = Cursor::new(invalid);
    let result = ObjLoader.load(&mut cursor, &LoadOptions::default());
    assert!(result.is_err(), "invalid UTF-8 should return an error");
}
