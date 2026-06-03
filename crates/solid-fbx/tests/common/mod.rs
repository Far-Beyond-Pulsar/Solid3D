//! Shared scene factories and round-trip helpers for solid-fbx integration tests.

#![allow(dead_code)]

use glam::*;
use solid_fbx::{FbxLoader, FbxSaver};
use solid_rs::prelude::*;
use std::io::Cursor;

// ── Minimal mesh factory ───────────────────────────────────────────────────────

/// Returns a minimal triangle mesh suitable for use in any scene.
pub fn make_minimal_mesh(name: &str) -> Mesh {
    let mut mesh = Mesh::new(name);
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    mesh
}

// ── Scene factories ────────────────────────────────────────────────────────────

/// Build a minimal triangle scene (3 verts, 1 primitive).
pub fn triangle_scene() -> Scene {
    let mut b = SceneBuilder::new();
    let mi = b.push_mesh(make_minimal_mesh("Triangle"));
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// Build a scene with a full PBR material.
pub fn material_scene() -> Scene {
    let mut b = SceneBuilder::new();
    let mut mat = Material::new("PBR");
    mat.base_color_factor = Vec4::new(0.8, 0.2, 0.1, 1.0);
    mat.roughness_factor = 0.4;
    mat.metallic_factor = 0.6;
    mat.emissive_factor = Vec3::new(0.1, 0.05, 0.0);
    mat.alpha_mode = AlphaMode::Opaque;
    let mat_idx = b.push_material(mat);

    let mut mesh = make_minimal_mesh("PBRMesh");
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], Some(mat_idx))];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// Build a scene with a camera (perspective or orthographic).
pub fn camera_scene(orthographic: bool) -> Scene {
    let mut b = SceneBuilder::new();
    let cam = if orthographic {
        Camera::orthographic("OrthoCamera")
    } else {
        Camera::perspective("PerspCamera")
    };
    let ci = b.push_camera(cam);
    let r = b.add_root_node("Root");
    let cn = b.add_child_node(r, "CamNode");
    b.attach_camera(cn, ci);
    b.build()
}

/// Build a scene containing one of each light type.
pub fn lights_scene() -> Scene {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("Root");

    let pt = Light::Point(PointLight {
        base: LightBase {
            name: "PointLight".into(),
            color: Vec3::new(1.0, 0.9, 0.8),
            intensity: 100.0,
        },
        range: Some(10.0),
        extensions: Extensions::new(),
    });
    let li0 = b.push_light(pt);
    let n0 = b.add_child_node(r, "PointNode");
    b.attach_light(n0, li0);

    let dir = Light::Directional(DirectionalLight {
        base: LightBase {
            name: "DirLight".into(),
            color: Vec3::ONE,
            intensity: 2.0,
        },
        extensions: Extensions::new(),
    });
    let li1 = b.push_light(dir);
    let n1 = b.add_child_node(r, "DirNode");
    b.attach_light(n1, li1);

    let spot = Light::Spot(SpotLight {
        base: LightBase {
            name: "SpotLight".into(),
            color: Vec3::new(0.8, 0.8, 1.0),
            intensity: 50.0,
        },
        range: Some(20.0),
        inner_cone_angle: 0.3,
        outer_cone_angle: 0.5,
        extensions: Extensions::new(),
    });
    let li2 = b.push_light(spot);
    let n2 = b.add_child_node(r, "SpotNode");
    b.attach_light(n2, li2);

    let area = Light::Area(AreaLight {
        base: LightBase {
            name: "AreaLight".into(),
            color: Vec3::new(1.0, 1.0, 0.9),
            intensity: 10.0,
        },
        width: 2.0,
        height: 1.5,
        extensions: Extensions::new(),
    });
    let li3 = b.push_light(area);
    let n3 = b.add_child_node(r, "AreaNode");
    b.attach_light(n3, li3);

    b.build()
}

/// Build a scene with a skin (2 joints, 4 vertices each with weights).
pub fn skinned_scene() -> Scene {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("Root");
    let joint0 = b.add_child_node(r, "Joint0");
    let joint1 = b.add_child_node(r, "Joint1");

    let skin = Skin {
        name: "MySkin".into(),
        skeleton_root: Some(joint0),
        joints: vec![joint0, joint1],
        inverse_bind_matrices: vec![Mat4::IDENTITY, Mat4::IDENTITY],
        extensions: Extensions::new(),
    };
    let si = b.push_skin(skin);

    let mut mesh = Mesh::new("SkinnedMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 0.0, 0.0))
            .with_normal(Vec3::Z)
            .with_skin_weights(SkinWeights {
                joints: [0, 1, 0, 0],
                weights: [1.00, 0.00, 0.0, 0.0],
            }),
        Vertex::new(Vec3::new(1.0, 0.0, 0.0))
            .with_normal(Vec3::Z)
            .with_skin_weights(SkinWeights {
                joints: [0, 1, 0, 0],
                weights: [0.50, 0.50, 0.0, 0.0],
            }),
        Vertex::new(Vec3::new(0.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_skin_weights(SkinWeights {
                joints: [0, 1, 0, 0],
                weights: [0.00, 1.00, 0.0, 0.0],
            }),
        Vertex::new(Vec3::new(1.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_skin_weights(SkinWeights {
                joints: [0, 1, 0, 0],
                weights: [0.25, 0.75, 0.0, 0.0],
            }),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2, 1, 3, 2], None)];
    let mi = b.push_mesh(mesh);
    let mesh_node = b.add_child_node(r, "MeshNode");
    b.attach_mesh(mesh_node, mi);
    b.attach_skin(mesh_node, si);
    b.build()
}

/// Build a scene with one animation (translation + scale channels).
pub fn animated_scene() -> Scene {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("Root");
    let target = b.add_child_node(r, "Animated");

    let anim = Animation {
        name: "Walk".into(),
        channels: vec![
            AnimationChannel {
                target: AnimationTarget::Translation(target),
                interpolation: Interpolation::Linear,
                times: vec![0.0, 1.0],
                values: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            },
            AnimationChannel {
                target: AnimationTarget::Scale(target),
                interpolation: Interpolation::Linear,
                times: vec![0.0, 1.0],
                values: vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0],
            },
        ],
        extensions: Extensions::new(),
    };
    b.push_animation(anim);
    b.build()
}

/// Build a scene with tangents on every vertex.
pub fn tangent_scene() -> Scene {
    let mut b = SceneBuilder::new();
    let mut mesh = Mesh::new("TangentMesh");

    let make_v = |pos: Vec3, uv: Vec2| {
        let mut v = Vertex::new(pos).with_normal(Vec3::Z).with_uv(uv);
        v.tangent = Some(Vec4::new(1.0, 0.0, 0.0, 1.0));
        v
    };
    mesh.vertices = vec![
        make_v(Vec3::new(0.0, 1.0, 0.0), Vec2::new(0.5, 0.5)),
        make_v(Vec3::new(-1.0, -1.0, 0.0), Vec2::new(0.0, 0.5)),
        make_v(Vec3::new(1.0, -1.0, 0.0), Vec2::new(1.0, 0.5)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

/// Build a scene with per-vertex colours (RGB primaries).
pub fn vertex_color_scene() -> Scene {
    let mut b = SceneBuilder::new();
    let mut mesh = Mesh::new("ColorMesh");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_color(Vec4::new(1.0, 0.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_color(Vec4::new(0.0, 1.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_color(Vec4::new(0.0, 0.0, 1.0, 1.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

// ── Round-trip helpers ─────────────────────────────────────────────────────────

/// Round-trip through ASCII FBX: scene → ASCII bytes → parse back → Scene.
pub fn ascii_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::new();
    FbxSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    let mut cursor = Cursor::new(buf);
    FbxLoader
        .load(&mut cursor, &LoadOptions::default())
        .unwrap()
}

/// Round-trip through binary FBX: scene → binary bytes → parse back → Scene.
pub fn binary_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::new();
    FbxSaver.save_binary(scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    FbxLoader
        .load(&mut cursor, &LoadOptions::default())
        .unwrap()
}
