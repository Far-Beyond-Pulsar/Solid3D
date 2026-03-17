//! Integration tests for GltfLoader.

mod common;

use common::*;
use glam::Vec3;
use solid_gltf::GltfLoader;
use solid_rs::prelude::*;
use solid_rs::scene::{AlphaMode, Projection};
use std::io::Cursor;

// ── Rejection tests ───────────────────────────────────────────────────────────

#[test]
fn loader_rejects_empty() {
    let result = GltfLoader.load(&mut Cursor::new(b""), &LoadOptions::default());
    assert!(result.is_err(), "expected error for empty input");
}

#[test]
fn loader_rejects_invalid_json() {
    let result = GltfLoader.load(&mut Cursor::new(b"not json at all!!!"), &LoadOptions::default());
    assert!(result.is_err(), "expected error for invalid JSON");
}

// ── Minimal valid glTF ────────────────────────────────────────────────────────

#[test]
fn loader_minimal_gltf_loads() {
    let json = r#"{"asset":{"version":"2.0"}}"#;
    let scene = GltfLoader
        .load(&mut Cursor::new(json.as_bytes()), &LoadOptions::default())
        .expect("minimal glTF should load");
    // No meshes / nodes is fine.
    let _ = scene;
}

// ── Vertex data ───────────────────────────────────────────────────────────────

#[test]
fn loader_triangle_vertex_count() {
    let scene = gltf_round_trip(&triangle_scene());
    assert_eq!(scene.meshes[0].vertices.len(), 3);
}

#[test]
fn loader_triangle_indices_correct() {
    let scene = gltf_round_trip(&triangle_scene());
    assert_eq!(scene.meshes[0].primitives[0].indices, vec![0, 1, 2]);
}

#[test]
fn loader_normals_loaded() {
    let scene = gltf_round_trip(&triangle_scene());
    let verts = &scene.meshes[0].vertices;
    assert!(verts.iter().all(|v| v.normal.is_some()), "normals should be present after round-trip");
    assert!(
        verts.iter().all(|v| {
            let n = v.normal.unwrap();
            (n - Vec3::Z).length() < 1e-5
        }),
        "normals should be Vec3::Z"
    );
}

#[test]
fn loader_uvs_loaded() {
    let scene = gltf_round_trip(&pbr_material_scene());
    let verts = &scene.meshes[0].vertices;
    assert!(verts.iter().all(|v| v.uvs[0].is_some()), "UV channel 0 should be present");
}

#[test]
fn loader_tangents_loaded() {
    // Build a scene with explicit tangents
    use glam::Vec4;
    use solid_rs::builder::SceneBuilder;
    use solid_rs::geometry::{Primitive, Vertex};
    let mut b = SceneBuilder::named("Tangent Scene");
    let mut mesh = solid_rs::scene::Mesh::new("T");
    mesh.vertices = vec![
        Vertex::new(Vec3::X).with_normal(Vec3::Z),
        Vertex::new(Vec3::Y).with_normal(Vec3::Z),
        Vertex::new(Vec3::Z).with_normal(Vec3::Z),
    ];
    // Manually set tangents
    for v in &mut mesh.vertices {
        v.tangent = Some(Vec4::new(1.0, 0.0, 0.0, 1.0));
    }
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("R");
    b.attach_mesh(r, mi);
    let original = b.build();

    let loaded = gltf_round_trip(&original);
    assert!(
        loaded.meshes[0].vertices.iter().all(|v| v.tangent.is_some()),
        "tangents should survive round-trip"
    );
}

#[test]
fn loader_vertex_colors_loaded() {
    let scene = gltf_round_trip(&pbr_material_scene());
    let verts = &scene.meshes[0].vertices;
    assert!(verts.iter().all(|v| v.colors[0].is_some()), "vertex colors should be present");
}

// ── Material properties ───────────────────────────────────────────────────────

#[test]
fn loader_material_base_color() {
    let scene = gltf_round_trip(&pbr_material_scene());
    let c = scene.materials[0].base_color_factor;
    assert!((c.x - 0.8).abs() < 1e-5, "base color R should be ~0.8, got {}", c.x);
    assert!((c.y - 0.2).abs() < 1e-5, "base color G should be ~0.2, got {}", c.y);
}

#[test]
fn loader_material_roughness() {
    let scene = gltf_round_trip(&pbr_material_scene());
    let r = scene.materials[0].roughness_factor;
    assert!((r - 0.7).abs() < 1e-5, "roughness should be ~0.7, got {r}");
}

#[test]
fn loader_material_metallic() {
    let scene = gltf_round_trip(&pbr_material_scene());
    let m = scene.materials[0].metallic_factor;
    assert!((m - 0.3).abs() < 1e-5, "metallic should be ~0.3, got {m}");
}

#[test]
fn loader_material_specular_ior_extensions() {
    let json = r#"{
        "asset": { "version": "2.0" },
        "materials": [
            {
                "name": "SpecMat",
                "extensions": {
                    "KHR_materials_specular": {
                        "specularFactor": 0.65,
                        "specularColorFactor": [0.9, 0.8, 0.7]
                    },
                    "KHR_materials_ior": {
                        "ior": 1.33
                    }
                }
            }
        ]
    }"#;

    let scene = GltfLoader
        .load(&mut Cursor::new(json.as_bytes()), &LoadOptions::default())
        .expect("glTF with specular/ior extensions should load");

    let material = &scene.materials[0];
    assert!((material.specular_weight - 0.65).abs() < 1e-5);
    assert!((material.specular_color.x - 0.9).abs() < 1e-5);
    assert!((material.specular_color.y - 0.8).abs() < 1e-5);
    assert!((material.specular_color.z - 0.7).abs() < 1e-5);
    assert!((material.ior - 1.33).abs() < 1e-5);
}

#[test]
fn loader_material_alpha_mode_opaque() {
    let scene = gltf_round_trip(&pbr_material_scene());
    assert_eq!(scene.materials[0].alpha_mode, AlphaMode::Opaque);
}

#[test]
fn loader_material_alpha_mode_mask() {
    use solid_rs::builder::SceneBuilder;
    use solid_rs::scene::Material;
    let mut b = SceneBuilder::named("Mask");
    let mut mat = Material::new("MaskMat");
    mat.alpha_mode = AlphaMode::Mask;
    mat.alpha_cutoff = 0.5;
    b.push_material(mat);
    b.add_root_node("R");
    let loaded = gltf_round_trip(&b.build());
    assert_eq!(loaded.materials[0].alpha_mode, AlphaMode::Mask);
}

#[test]
fn loader_material_alpha_mode_blend() {
    use solid_rs::builder::SceneBuilder;
    use solid_rs::scene::Material;
    let mut b = SceneBuilder::named("Blend");
    let mut mat = Material::new("BlendMat");
    mat.alpha_mode = AlphaMode::Blend;
    b.push_material(mat);
    b.add_root_node("R");
    let loaded = gltf_round_trip(&b.build());
    assert_eq!(loaded.materials[0].alpha_mode, AlphaMode::Blend);
}

#[test]
fn loader_material_double_sided() {
    let scene = gltf_round_trip(&pbr_material_scene());
    assert!(scene.materials[0].double_sided, "double_sided should round-trip");
}

// ── Node hierarchy & transforms ───────────────────────────────────────────────

#[test]
fn loader_node_hierarchy_depth() {
    let original = camera_scene();
    let scene = gltf_round_trip(&original);
    // World node has 2 camera children
    let root = scene.node(*scene.roots.first().unwrap()).unwrap();
    assert_eq!(root.children.len(), 2, "root should have 2 child nodes");
}

#[test]
fn loader_node_translation() {
    use solid_rs::builder::SceneBuilder;
    use solid_rs::geometry::Transform;
    let mut b = SceneBuilder::named("Trans");
    let r = b.add_root_node("R");
    b.set_transform(r, Transform::IDENTITY.with_translation(glam::Vec3::new(1.0, 2.0, 3.0)));
    let loaded = gltf_round_trip(&b.build());
    let t = loaded.node(*loaded.roots.first().unwrap()).unwrap().transform.translation;
    assert!((t.x - 1.0).abs() < 1e-5);
    assert!((t.y - 2.0).abs() < 1e-5);
    assert!((t.z - 3.0).abs() < 1e-5);
}

// ── Cameras ───────────────────────────────────────────────────────────────────

#[test]
fn loader_perspective_camera() {
    let scene = gltf_round_trip(&camera_scene());
    let cam = &scene.cameras[0];
    match &cam.projection {
        Projection::Perspective(p) => {
            assert!((p.fov_y - 0.785398).abs() < 1e-4, "fov_y mismatch: {}", p.fov_y);
        }
        _ => panic!("expected perspective camera"),
    }
}

#[test]
fn loader_orthographic_camera() {
    let scene = gltf_round_trip(&camera_scene());
    let cam = &scene.cameras[1];
    match &cam.projection {
        Projection::Orthographic(o) => {
            assert!((o.x_mag - 5.0).abs() < 1e-5, "x_mag mismatch: {}", o.x_mag);
        }
        _ => panic!("expected orthographic camera"),
    }
}

// ── GLB binary format ─────────────────────────────────────────────────────────

#[test]
fn loader_glb_magic_detected() {
    let mut buf = Vec::<u8>::new();
    solid_gltf::GltfSaver.save_glb(&triangle_scene(), &mut buf).unwrap();
    assert_eq!(&buf[0..4], b"glTF", "GLB magic bytes mismatch");
}

#[test]
fn loader_glb_triangle_vertex_count() {
    let original = triangle_scene();
    let loaded = glb_round_trip(&original);
    assert_eq!(loaded.meshes[0].vertices.len(), 3);
}
