//! Integration tests for StlSaver (binary and ASCII).

mod common;

use common::*;
use solid_rs::prelude::*;
use solid_stl::StlSaver;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Save a scene as binary STL and return the raw bytes.
fn save_binary(scene: &Scene) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    StlSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    buf
}

/// Save a scene as ASCII STL and return the raw bytes.
fn save_ascii_bytes(scene: &Scene) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    StlSaver
        .save_ascii(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    buf
}

fn std_scene() -> Scene {
    triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 1.0, 0.0),
    )
}

// ── Binary – structure ────────────────────────────────────────────────────────

#[test]
fn saver_binary_header_80_bytes() {
    let scene = {
        let mut b = SceneBuilder::named("S");
        let mut mesh = Mesh::new("MyHeader");
        mesh.vertices = vec![
            Vertex::new(glam::Vec3::new(0.0, 0.0, 0.0)),
            Vertex::new(glam::Vec3::new(1.0, 0.0, 0.0)),
            Vertex::new(glam::Vec3::new(0.0, 1.0, 0.0)),
        ];
        mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
        let mi = b.push_mesh(mesh);
        let r = b.add_root_node("R");
        b.attach_mesh(r, mi);
        b.build()
    };
    let buf = save_binary(&scene);
    // First 80 bytes form the header; must start with the mesh name.
    let header = std::str::from_utf8(&buf[..80]).unwrap();
    assert!(
        header.starts_with("MyHeader"),
        "header should start with mesh name"
    );
}

#[test]
fn saver_binary_triangle_count_correct() {
    let buf = save_binary(&std_scene());
    let count = u32::from_le_bytes([buf[80], buf[81], buf[82], buf[83]]);
    assert_eq!(count, 1, "triangle count field should be 1");
}

#[test]
fn saver_binary_total_size_correct() {
    let buf = save_binary(&std_scene());
    assert_eq!(
        buf.len(),
        80 + 4 + 1 * 50,
        "total binary size should be 134 bytes"
    );
}

#[test]
fn saver_binary_positions_correct() {
    let buf = save_binary(&std_scene());
    // Triangle 0: normal at 84, v0 at 96, v1 at 108, v2 at 120
    let v0 = read_vec3(&buf, 96);
    let v1 = read_vec3(&buf, 108);
    let v2 = read_vec3(&buf, 120);
    assert_eq!(v0, glam::Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(v1, glam::Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(v2, glam::Vec3::new(0.0, 1.0, 0.0));
}

// ── Binary – normals ──────────────────────────────────────────────────────────

#[test]
fn saver_binary_face_normals_computed() {
    let buf = save_binary(&std_scene());
    // Triangle in XY plane → normal should be (0,0,1)
    let normal = read_vec3(&buf, 84);
    assert!((normal.x).abs() < 1e-5);
    assert!((normal.y).abs() < 1e-5);
    assert!((normal.z - 1.0).abs() < 1e-5);
}

#[test]
fn saver_binary_face_normals_unit_length() {
    // Use a non-axis-aligned triangle to exercise the normalisation path.
    let scene = triangle_scene(
        glam::Vec3::new(0.0, 0.0, 0.0),
        glam::Vec3::new(2.0, 0.0, 0.0),
        glam::Vec3::new(0.0, 3.0, 1.0),
    );
    let buf = save_binary(&scene);
    let normal = read_vec3(&buf, 84);
    let len = normal.length();
    assert!(
        (len - 1.0).abs() < 1e-5,
        "face normal length should be 1.0, got {len}"
    );
}

// ── Binary – VisCAM color ─────────────────────────────────────────────────────

#[test]
fn saver_binary_viscam_color_encoded() {
    let scene = colored_triangle_scene(glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
    let buf = save_binary(&scene);
    let attr = read_u16_le(&buf, 132); // attr bytes at offset 80+4+48
                                       // bit 15 must be set
    assert!(attr & 0x8000 != 0, "color-valid bit (15) must be set");
    let r5 = (attr >> 10) & 0x1F;
    let g5 = (attr >> 5) & 0x1F;
    let b5 = attr & 0x1F;
    assert_eq!(r5, 31, "R channel should be 31");
    assert_eq!(g5, 0, "G channel should be 0");
    assert_eq!(b5, 0, "B channel should be 0");
}

#[test]
fn saver_binary_no_color_when_none() {
    let buf = save_binary(&std_scene());
    let attr = read_u16_le(&buf, 132);
    assert_eq!(attr, 0, "attr bytes should be 0 when no color is set");
}

// ── ASCII – structure ─────────────────────────────────────────────────────────

#[test]
fn saver_ascii_starts_with_solid() {
    let bytes = save_ascii_bytes(&std_scene());
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(
        text.starts_with("solid "),
        "ASCII STL must start with 'solid '"
    );
}

#[test]
fn saver_ascii_ends_with_endsolid() {
    let bytes = save_ascii_bytes(&std_scene());
    let text = std::str::from_utf8(&bytes).unwrap().trim_end();
    assert!(
        text.ends_with("endsolid Triangle"),
        "ASCII STL must end with 'endsolid <name>'"
    );
}

#[test]
fn saver_ascii_vertex_lines_present() {
    let bytes = save_ascii_bytes(&std_scene());
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(
        text.contains("vertex "),
        "ASCII STL must contain 'vertex' lines"
    );
    // One triangle → exactly 3 vertex lines
    let count = text
        .lines()
        .filter(|l| l.trim().starts_with("vertex "))
        .count();
    assert_eq!(
        count, 3,
        "one triangle should produce exactly 3 vertex lines"
    );
}

#[test]
fn saver_ascii_normal_lines_present() {
    let bytes = save_ascii_bytes(&std_scene());
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(
        text.contains("facet normal "),
        "ASCII STL must contain 'facet normal' lines"
    );
}

// ── ASCII – multiple meshes ───────────────────────────────────────────────────

fn two_mesh_scene() -> Scene {
    let mut b = SceneBuilder::named("Two Meshes");
    let mut m1 = Mesh::new("MeshA");
    m1.vertices = vec![
        Vertex::new(glam::Vec3::new(0.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(1.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(0.0, 1.0, 0.0)),
    ];
    m1.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mut m2 = Mesh::new("MeshB");
    m2.vertices = vec![
        Vertex::new(glam::Vec3::new(2.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(3.0, 0.0, 0.0)),
        Vertex::new(glam::Vec3::new(2.0, 1.0, 0.0)),
    ];
    m2.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    let mi1 = b.push_mesh(m1);
    let mi2 = b.push_mesh(m2);
    let r1 = b.add_root_node("R1");
    let r2 = b.add_root_node("R2");
    b.attach_mesh(r1, mi1);
    b.attach_mesh(r2, mi2);
    b.build()
}

#[test]
fn saver_ascii_multiple_meshes() {
    let bytes = save_ascii_bytes(&two_mesh_scene());
    let text = std::str::from_utf8(&bytes).unwrap();
    // Each mesh produces one "solid <name>" line.
    let solid_count = text.lines().filter(|l| l.starts_with("solid ")).count();
    assert_eq!(
        solid_count, 2,
        "two meshes should produce two 'solid' blocks"
    );
    let endsolid_count = text.lines().filter(|l| l.starts_with("endsolid ")).count();
    assert_eq!(
        endsolid_count, 2,
        "two meshes should produce two 'endsolid' lines"
    );
}

#[test]
fn saver_binary_multiple_meshes() {
    let buf = save_binary(&two_mesh_scene());
    let count = u32::from_le_bytes([buf[80], buf[81], buf[82], buf[83]]);
    assert_eq!(
        count, 2,
        "two meshes with 1 triangle each → total count = 2"
    );
    assert_eq!(buf.len(), 80 + 4 + 2 * 50);
}

// ── Empty scene ───────────────────────────────────────────────────────────────

#[test]
fn saver_empty_scene_no_crash() {
    let scene = Scene::new();
    // Binary
    let buf = save_binary(&scene);
    assert_eq!(
        buf.len(),
        84,
        "empty binary STL should be 84 bytes (header + count)"
    );
    let count = u32::from_le_bytes([buf[80], buf[81], buf[82], buf[83]]);
    assert_eq!(count, 0);

    // ASCII
    let bytes = save_ascii_bytes(&scene);
    assert!(bytes.is_empty(), "empty scene ASCII output should be empty");
}
