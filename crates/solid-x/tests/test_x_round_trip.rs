use solid_rs::prelude::*;
use solid_x::{XLoader, XSaver};
use std::io::Cursor;

fn triangle_scene() -> Scene {
    let mut builder = SceneBuilder::named("triangle");
    let mut mesh = Mesh::new("tri");
    mesh.vertices = vec![
        Vertex::new(glam::Vec3::new(0.0, 0.0, 0.0)).with_normal(glam::Vec3::Z),
        Vertex::new(glam::Vec3::new(1.0, 0.0, 0.0)).with_normal(glam::Vec3::Z),
        Vertex::new(glam::Vec3::new(0.0, 1.0, 0.0)).with_normal(glam::Vec3::Z),
    ];
    mesh.primitives
        .push(Primitive::triangles(vec![0, 1, 2], None));
    mesh.compute_bounds();
    let mesh_idx = builder.push_mesh(mesh);
    let root = builder.add_root_node("root");
    builder.attach_mesh(root, mesh_idx);
    builder.build()
}

#[test]
fn round_trip_x_ascii() {
    let scene = triangle_scene();
    let mut buf = Vec::new();
    XSaver
        .save(&scene, &mut buf, &SaveOptions::default())
        .expect("failed to save .x");
    let loaded = XLoader
        .load(&mut Cursor::new(buf), &LoadOptions::default())
        .expect("failed to reload .x");

    assert_eq!(loaded.meshes.len(), 1);
    assert_eq!(loaded.meshes[0].vertices.len(), 3);
    assert_eq!(loaded.meshes[0].primitives.len(), 1);
    assert_eq!(loaded.meshes[0].primitives[0].indices, vec![0, 1, 2]);
}
