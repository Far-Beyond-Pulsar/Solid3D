//! Demonstrates loading (or inspecting) a 3D scene via the format registry.
//!
//! In a real programme you would add a format crate dependency and register
//! its loader.  This example builds a scene in-memory to show the same
//! inspection code.

use glam::Vec3;
use solid_rs::prelude::*;

fn main() -> Result<()> {
    // ── 1. Build a registry and register format crates ────────────────────
    let registry = Registry::new();
    // registry.register_loader(solid_obj::ObjLoader::default());
    // registry.register_loader(solid_gltf::GltfLoader::default());

    // Print everything the registry can currently load.
    println!("Registered loaders:");
    let mut any = false;
    for info in registry.loader_infos() {
        println!(
            "  [{id}]  {name}  ({exts})",
            id = info.id,
            name = info.name,
            exts = info.extensions.join(", ")
        );
        any = true;
    }
    if !any {
        println!("  (none — add a format crate to your Cargo.toml)");
    }

    // ── 2. Build a demo scene in-memory ───────────────────────────────────
    let scene = build_demo_scene();

    // ── 3. Inspect the scene ──────────────────────────────────────────────
    println!("\nScene: \"{}\"", scene.name);
    println!("  nodes      : {}", scene.nodes.len());
    println!("  meshes     : {}", scene.meshes.len());
    println!("  materials  : {}", scene.materials.len());
    println!("  textures   : {}", scene.textures.len());
    println!("  animations : {}", scene.animations.len());
    println!("  total verts: {}", scene.total_vertex_count());

    println!("\nNode hierarchy:");
    for &root_id in &scene.roots {
        print_node(&scene, root_id, 0);
    }

    Ok(())
}

fn print_node(scene: &solid_rs::Scene, id: NodeId, depth: usize) {
    let indent = "  ".repeat(depth);
    let node = scene.node(id).unwrap();
    let mesh_info = node
        .mesh
        .map(|i| format!("  [mesh: {}]", scene.meshes[i].name))
        .unwrap_or_default();
    println!("{}▸ {}{}", indent, node.name, mesh_info);
    for &child in &node.children {
        print_node(scene, child, depth + 1);
    }
}

// ── Demo scene construction ───────────────────────────────────────────────────

fn build_demo_scene() -> solid_rs::Scene {
    let mut b = SceneBuilder::named("Demo Scene");

    // Mesh: a simple triangle.
    let mut tri = Mesh::new("Triangle");
    tri.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_normal(Vec3::Z),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_normal(Vec3::Z),
    ];
    tri.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let tri_idx = b.push_mesh(tri);

    // Node hierarchy: Root → Pivot → Triangle.
    let root = b.add_root_node("Root");
    let pivot = b.add_child_node(root, "Pivot");
    let leaf = b.add_child_node(pivot, "TriangleNode");
    b.attach_mesh(leaf, tri_idx);

    b.build()
}
