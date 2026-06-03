mod common;
use common::*;
use glam::{Vec3, Vec4};
use solid_obj::ObjSaver;
use solid_rs::prelude::*;
use solid_rs::scene::AlphaMode;

// ── Geometry lines ────────────────────────────────────────────────────────────

#[test]
fn saver_triangle_produces_v_lines() {
    let text = save_obj_to_string(&triangle_scene());
    let v_lines = text.lines().filter(|l| l.starts_with("v ")).count();
    assert_eq!(v_lines, 3, "expected 3 'v' lines for a triangle");
}

#[test]
fn saver_normals_produce_vn_lines() {
    let text = save_obj_to_string(&triangle_scene());
    let vn_lines = text.lines().filter(|l| l.starts_with("vn ")).count();
    assert_eq!(
        vn_lines, 3,
        "expected 3 'vn' lines for a triangle with normals"
    );
}

#[test]
fn saver_uvs_produce_vt_lines() {
    let text = save_obj_to_string(&material_scene());
    let vt_lines = text.lines().filter(|l| l.starts_with("vt ")).count();
    assert_eq!(vt_lines, 3, "expected 3 'vt' lines for a triangle with UVs");
}

#[test]
fn saver_face_line_format() {
    // Triangle with no UVs or normals → "f i j k" format
    let mut b = SceneBuilder::new();
    let mut mesh = Mesh::new("Plain");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Plain");
    b.attach_mesh(r, mi);
    let scene = b.build();

    let text = save_obj_to_string(&scene);
    let face_line = text
        .lines()
        .find(|l| l.starts_with("f "))
        .expect("no face line");
    // Should be "f 1 2 3" — no slashes when no UVs/normals
    assert_eq!(face_line, "f 1 2 3");
}

// ── Material lines ────────────────────────────────────────────────────────────

#[test]
fn saver_mtllib_line_present() {
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("mtllib scene.mtl"),
        "output should reference 'scene.mtl'"
    );
}

#[test]
fn saver_usemtl_line_present() {
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("usemtl "),
        "output should contain a usemtl directive"
    );
}

#[test]
fn saver_mtl_kd_emitted() {
    // material_scene has base_color_factor = (0.8, 0.4, 0.2, 1.0)
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("Kd 0.8000 0.4000 0.2000"),
        "Kd line not found; output:\n{}",
        text
    );
}

#[test]
fn saver_mtl_ke_emitted() {
    // material_scene emissive = (0.1, 0.2, 0.3)
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("Ke 0.1000 0.2000 0.3000"),
        "Ke line not found; output:\n{}",
        text
    );
}

#[test]
fn saver_mtl_ns_emitted() {
    // roughness = 0.7 → Ns = (1-0.7)^2 * 1000 = 0.09 * 1000 = 90.0
    let text = save_obj_to_string(&material_scene());
    assert!(text.contains("Ns "), "Ns line not found; output:\n{}", text);
    let ns_line = text.lines().find(|l| l.starts_with("Ns ")).unwrap();
    let ns_val: f32 = ns_line.split_whitespace().nth(1).unwrap().parse().unwrap();
    assert!((ns_val - 90.0).abs() < 1.0, "Ns value mismatch: {}", ns_val);
}

#[test]
fn saver_mtl_d_opaque_not_emitted_or_is_one() {
    // For AlphaMode::Opaque (default), 'd' should NOT be emitted
    let _text = save_obj_to_string(&material_scene()); // material_scene has opaque material
                                                       // Re-check: material_scene has alpha = 1.0, alpha_mode = Opaque
    let scene2 = {
        let mut b = SceneBuilder::new();
        let mat = Material::solid_color("Opaque", Vec4::ONE);
        let mi_m = b.push_material(mat);
        let mut mesh = Mesh::new("M");
        mesh.vertices = vec![
            Vertex::new(Vec3::ZERO),
            Vertex::new(Vec3::X),
            Vertex::new(Vec3::Y),
        ];
        mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi_m))];
        let mi = b.push_mesh(mesh);
        let r = b.add_root_node("M");
        b.attach_mesh(r, mi);
        b.build()
    };
    let text2 = save_obj_to_string(&scene2);
    let has_d_directive = text2.lines().any(|l| l.trim_start().starts_with("d "));
    assert!(
        !has_d_directive,
        "'d' should not be emitted for opaque materials"
    );
}

#[test]
fn saver_mtl_d_blend_emitted() {
    let mut b = SceneBuilder::new();
    let mut mat = Material::new("Blend");
    mat.base_color_factor = Vec4::new(1.0, 1.0, 1.0, 0.5);
    mat.alpha_mode = AlphaMode::Blend;
    let mi_m = b.push_material(mat);
    let mut mesh = Mesh::new("M");
    mesh.vertices = vec![
        Vertex::new(Vec3::ZERO),
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi_m))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("M");
    b.attach_mesh(r, mi);
    let scene = b.build();

    let text = save_obj_to_string(&scene);
    assert!(
        text.contains("d 0.5000"),
        "'d 0.5000' not found for blend material; output:\n{}",
        text
    );
}

#[test]
fn saver_mtl_d_mask_emitted() {
    let mut b = SceneBuilder::new();
    let mut mat = Material::new("Mask");
    mat.alpha_mode = AlphaMode::Mask;
    mat.alpha_cutoff = 0.3;
    let mi_m = b.push_material(mat);
    let mut mesh = Mesh::new("M");
    mesh.vertices = vec![
        Vertex::new(Vec3::ZERO),
        Vertex::new(Vec3::X),
        Vertex::new(Vec3::Y),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi_m))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("M");
    b.attach_mesh(r, mi);
    let scene = b.build();

    let text = save_obj_to_string(&scene);
    // For mask, d = 1.0 - alpha_cutoff = 0.7
    assert!(
        text.contains("d 0.7000"),
        "'d 0.7000' not found for mask material; output:\n{}",
        text
    );
}

#[test]
fn saver_mtl_pr_emitted() {
    // material_scene has roughness_factor = 0.7
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("Pr 0.7000"),
        "Pr line not found; output:\n{}",
        text
    );
}

#[test]
fn saver_mtl_pm_emitted() {
    // material_scene has metallic_factor = 0.3
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("Pm 0.3000"),
        "Pm line not found; output:\n{}",
        text
    );
}

#[test]
fn saver_mtl_map_ke_emitted() {
    // material_scene has emissive_texture → emissive.png
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("map_Ke emissive.png"),
        "map_Ke not found; output:\n{}",
        text
    );
}

#[test]
fn saver_mtl_norm_emitted() {
    // material_scene has normal_texture → normal.png
    let text = save_obj_to_string(&material_scene());
    assert!(
        text.contains("norm normal.png") || text.contains("map_bump normal.png"),
        "norm/map_bump not found; output:\n{}",
        text
    );
}

// ── Smoothing groups ──────────────────────────────────────────────────────────

#[test]
fn saver_smoothing_group_s_directive_emitted() {
    let text = save_obj_to_string(&triangle_scene());
    assert!(
        text.contains("s 1"),
        "'s 1' smoothing group directive not found"
    );
}

#[test]
fn saver_smoothing_group_s_off_after_primitive() {
    let text = save_obj_to_string(&triangle_scene());
    assert!(
        text.contains("s off"),
        "'s off' directive not found after primitive"
    );
}

// ── Multiple meshes ───────────────────────────────────────────────────────────

#[test]
fn saver_multiple_meshes() {
    let text = save_obj_to_string(&multi_mesh_scene());
    let g_lines = text.lines().filter(|l| l.starts_with("g ")).count();
    assert_eq!(
        g_lines, 2,
        "expected 2 'g' lines for two meshes; got {}",
        g_lines
    );
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn saver_empty_scene_no_crash() {
    let scene = Scene::new();
    let mut buf = Vec::new();
    let result = ObjSaver.save(&scene, &mut buf, &SaveOptions::default());
    assert!(result.is_ok(), "saving an empty scene should not error");
}
