//! Shared test helpers for solid-gltf integration tests.

#![allow(dead_code)]

use glam::{Mat4, Vec2, Vec3, Vec4};
use solid_gltf::{GltfLoader, GltfSaver};
use solid_rs::extensions::Extensions;
use solid_rs::geometry::{Primitive, SkinWeights, Vertex};
use solid_rs::prelude::*;
use solid_rs::scene::{
    AlphaMode, Animation, AnimationChannel, AnimationTarget, Camera, DirectionalLight,
    Interpolation, Light, LightBase, Material, Mesh, MorphTarget,
    OrthographicCamera, PerspectiveCamera, PointLight, Projection, Skin, SpotLight,
};
use solid_rs::builder::SceneBuilder;
use std::io::Cursor;

// ── Scene factories ───────────────────────────────────────────────────────────

/// Minimal triangle scene with normals.
pub fn triangle_scene() -> Scene {
    let mut b = SceneBuilder::named("Triangle Scene");
    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// Scene with a PBR material, UVs, tangents, and vertex colors.
pub fn pbr_material_scene() -> Scene {
    let mut b = SceneBuilder::named("PBR Scene");
    let mut mat = Material::new("PBR_Mat");
    mat.base_color_factor = Vec4::new(0.8, 0.2, 0.1, 1.0);
    mat.metallic_factor = 0.3;
    mat.roughness_factor = 0.7;
    mat.emissive_factor = Vec3::new(0.1, 0.0, 0.0);
    mat.alpha_mode = AlphaMode::Opaque;
    mat.double_sided = true;
    let mi_mat = b.push_material(mat);

    let mut mesh = Mesh::new("PBR_Mesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.5, 1.0))
            .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.0, 0.0))
            .with_color(Vec4::new(0.0, 1.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(1.0, 0.0))
            .with_color(Vec4::new(0.0, 0.0, 1.0, 1.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mi_mat))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// Scene with explicit specular/IOR terms alongside metallic/roughness.
pub fn pbr_specular_ior_scene() -> Scene {
    let mut scene = pbr_material_scene();
    let mat = &mut scene.materials[0];
    mat.specular_color = Vec3::new(0.9, 0.8, 0.7);
    mat.specular_weight = 0.65;
    mat.ior = 1.33;
    scene
}

/// Scene with perspective and orthographic cameras in a hierarchy.
pub fn camera_scene() -> Scene {
    let mut b = SceneBuilder::named("Camera Scene");
    let persp = Camera {
        name: "MainCam".into(),
        projection: Projection::Perspective(PerspectiveCamera {
            fov_y: 0.785398, // ~45 degrees
            aspect_ratio: Some(1.777),
            z_near: 0.1,
            z_far: Some(100.0),
        }),
        extensions: Extensions::new(),
    };
    let ortho = Camera {
        name: "OrthoView".into(),
        projection: Projection::Orthographic(OrthographicCamera {
            x_mag: 5.0,
            y_mag: 5.0,
            z_near: 0.1,
            z_far: 50.0,
        }),
        extensions: Extensions::new(),
    };
    let ci0 = b.push_camera(persp);
    let ci1 = b.push_camera(ortho);
    let root = b.add_root_node("World");
    let cn0 = b.add_child_node(root, "PerspNode");
    b.attach_camera(cn0, ci0);
    let cn1 = b.add_child_node(root, "OrthoNode");
    b.attach_camera(cn1, ci1);
    b.build()
}

/// Scene with a simple two-joint skin.
pub fn skinned_scene() -> Scene {
    let mut b = SceneBuilder::named("Skinned Scene");

    // Create two joint nodes
    let root = b.add_root_node("Armature");
    let j0 = b.add_child_node(root, "Joint0");
    let j1 = b.add_child_node(j0, "Joint1");

    let skin = Skin {
        name: "Rig".into(),
        skeleton_root: None,
        joints: vec![j0, j1],
        inverse_bind_matrices: vec![Mat4::IDENTITY, Mat4::IDENTITY],
        extensions: Extensions::new(),
    };
    let si = b.push_skin(skin);

    // Mesh with skin weights
    let mut mesh = Mesh::new("SkinMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::ZERO).with_skin_weights(SkinWeights {
            joints: [0, 1, 0, 0],
            weights: [0.6, 0.4, 0.0, 0.0],
        }),
        Vertex::new(Vec3::Y).with_skin_weights(SkinWeights {
            joints: [0, 1, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        }),
        Vertex::new(Vec3::X).with_skin_weights(SkinWeights {
            joints: [1, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        }),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let mesh_node = b.add_child_node(root, "MeshNode");
    b.attach_mesh(mesh_node, mi);
    b.attach_skin(mesh_node, si);
    b.build()
}

/// Scene with a translation + rotation animation.
pub fn animated_scene() -> Scene {
    let mut b = SceneBuilder::named("Animated Scene");
    let root = b.add_root_node("AnimRoot");

    let anim = Animation {
        name: "Bounce".into(),
        channels: vec![
            AnimationChannel {
                target: AnimationTarget::Translation(root),
                interpolation: Interpolation::Linear,
                times: vec![0.0, 1.0],
                values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            },
            AnimationChannel {
                target: AnimationTarget::Rotation(root),
                interpolation: Interpolation::Linear,
                times: vec![0.0, 1.0],
                values: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            },
        ],
        extensions: Extensions::new(),
    };
    b.push_animation(anim);
    b.build()
}

/// Scene with a mesh having 2 morph targets.
pub fn morph_target_scene() -> Scene {
    let mut b = SceneBuilder::named("Morph Scene");
    let mut mesh = Mesh::new("MorphMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    mesh.morph_targets = vec![
        MorphTarget {
            name: "smile".into(),
            position_deltas: vec![Vec3::Y * 0.1; 3],
            normal_deltas: vec![],
            tangent_deltas: vec![],
        },
        MorphTarget {
            name: "frown".into(),
            position_deltas: vec![Vec3::NEG_Y * 0.1; 3],
            normal_deltas: vec![],
            tangent_deltas: vec![],
        },
    ];
    mesh.morph_weights = vec![0.0, 0.5];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// Scene with point, directional and spot lights (KHR_lights_punctual).
pub fn lights_scene() -> Scene {
    let mut b = SceneBuilder::named("Lights Scene");
    let root = b.add_root_node("LightsRoot");

    let point = Light::Point(PointLight {
        base: LightBase { name: "Sun".into(), color: Vec3::ONE, intensity: 200.0 },
        range: Some(50.0),
        extensions: Extensions::new(),
    });
    let dir = Light::Directional(DirectionalLight {
        base: LightBase { name: "Moon".into(), color: Vec3::new(0.5, 0.5, 1.0), intensity: 1.0 },
        extensions: Extensions::new(),
    });
    let spot = Light::Spot(SpotLight {
        base: LightBase { name: "Flashlight".into(), color: Vec3::new(1.0, 1.0, 0.9), intensity: 500.0 },
        range: Some(20.0),
        inner_cone_angle: 0.2,
        outer_cone_angle: 0.4,
        extensions: Extensions::new(),
    });

    let li0 = b.push_light(point);
    let li1 = b.push_light(dir);
    let li2 = b.push_light(spot);
    let n0 = b.add_child_node(root, "PointNode");
    b.attach_light(n0, li0);
    let n1 = b.add_child_node(root, "DirNode");
    b.attach_light(n1, li1);
    let n2 = b.add_child_node(root, "SpotNode");
    b.attach_light(n2, li2);
    b.build()
}

// ── Round-trip helpers ────────────────────────────────────────────────────────

/// Save as glTF JSON, then reload.
pub fn gltf_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::<u8>::new();
    GltfSaver.save(scene, &mut buf, &SaveOptions::default()).expect("gltf save failed");
    GltfLoader
        .load(&mut Cursor::new(&buf), &LoadOptions::default())
        .expect("gltf load failed")
}

/// Save as GLB, then reload.
pub fn glb_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::<u8>::new();
    GltfSaver.save_glb(scene, &mut buf).expect("glb save failed");
    GltfLoader
        .load(&mut Cursor::new(&buf), &LoadOptions::default())
        .expect("glb load failed")
}
