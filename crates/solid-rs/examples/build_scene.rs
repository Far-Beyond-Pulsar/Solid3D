//! Demonstrates building a rich scene from scratch using [`SceneBuilder`].
//!
//! Covers: multiple meshes, materials, textures, a camera, a light, and an
//! animation clip.

use glam::{Quat, Vec2, Vec3, Vec4};
use solid_rs::prelude::*;
use std::f32::consts::PI;

fn main() -> Result<()> {
    let scene = build_rich_scene();

    println!("Scene: \"{}\"", scene.name);
    println!("  nodes      : {}", scene.nodes.len());
    println!("  meshes     : {}", scene.meshes.len());
    println!("  materials  : {}", scene.materials.len());
    println!("  textures   : {}", scene.textures.len());
    println!("  images     : {}", scene.images.len());
    println!("  cameras    : {}", scene.cameras.len());
    println!("  lights     : {}", scene.lights.len());
    println!("  animations : {}", scene.animations.len());

    println!("\nAnimations:");
    for anim in &scene.animations {
        println!(
            "  \"{}\"  {:.2}s  ({} channels)",
            anim.name,
            anim.duration(),
            anim.channels.len()
        );
    }

    println!("\nBounds:");
    for mesh in &scene.meshes {
        if let Some(aabb) = &mesh.bounds {
            println!(
                "  \"{}\"  min={:?}  max={:?}",
                mesh.name, aabb.min, aabb.max
            );
        }
    }

    Ok(())
}

fn build_rich_scene() -> solid_rs::Scene {
    let mut b = SceneBuilder::named("Rich Demo Scene");

    // ── Images & Textures ─────────────────────────────────────────────────
    let img_idx = b.push_image(Image::from_uri("albedo", "textures/albedo.png"));
    let tex_idx = b.push_texture(Texture::new("AlbedoTex", img_idx));

    // ── Material ──────────────────────────────────────────────────────────
    let mut mat = Material::new("PhysicalMat");
    mat.base_color_texture = Some(TextureRef::new(tex_idx));
    mat.metallic_factor = 0.0;
    mat.roughness_factor = 0.5;
    let mat_idx = b.push_material(mat);

    // ── Mesh: textured quad ───────────────────────────────────────────────
    let mut quad = Mesh::new("Quad");
    quad.vertices = vec![
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-1.0, 1.0, 0.0))
            .with_normal(Vec3::Z)
            .with_uv(Vec2::new(0.0, 1.0)),
    ];
    quad.primitives = vec![Primitive::triangles(vec![0, 1, 2, 0, 2, 3], Some(mat_idx))];
    let mut quad = quad;
    quad.compute_bounds();
    let quad_idx = b.push_mesh(quad);

    // ── Camera ────────────────────────────────────────────────────────────
    let cam_idx = b.push_camera(Camera::perspective("MainCamera"));

    // ── Light ─────────────────────────────────────────────────────────────
    let sun = Light::Directional(solid_rs::scene::DirectionalLight {
        base: solid_rs::scene::LightBase {
            name: "Sun".into(),
            color: Vec3::new(1.0, 0.95, 0.85),
            intensity: 3.0,
        },
        extensions: Extensions::new(),
    });
    let light_idx = b.push_light(sun);

    // ── Node hierarchy ────────────────────────────────────────────────────
    let root = b.add_root_node("World");
    let mesh_node = b.add_child_node(root, "QuadNode");
    b.attach_mesh(mesh_node, quad_idx);

    let cam_node = b.add_child_node(root, "CameraNode");
    b.set_transform(
        cam_node,
        Transform::IDENTITY.with_translation(Vec3::new(0.0, 0.0, 5.0)),
    );
    b.attach_camera(cam_node, cam_idx);

    let light_node = b.add_child_node(root, "SunNode");
    b.set_transform(
        light_node,
        Transform::IDENTITY.with_rotation(Quat::from_rotation_x(-PI / 4.0)),
    );
    b.attach_light(light_node, light_idx);

    // ── Animation: rotate the quad ────────────────────────────────────────
    let mut spin = Animation::new("Spin");
    spin.channels.push(AnimationChannel {
        target: AnimationTarget::Rotation(mesh_node),
        interpolation: Interpolation::Linear,
        times: vec![0.0, 1.0, 2.0],
        values: {
            // Keyframe quaternions (xyzw): identity → 180° → 360°
            let q0 = Quat::IDENTITY;
            let q1 = Quat::from_rotation_y(PI);
            let q2 = Quat::from_rotation_y(2.0 * PI);
            vec![
                q0.x, q0.y, q0.z, q0.w, q1.x, q1.y, q1.z, q1.w, q2.x, q2.y, q2.z, q2.w,
            ]
        },
    });
    b.push_animation(spin);

    b.build()
}
