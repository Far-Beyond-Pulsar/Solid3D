//! Shared helpers, mock format implementations, and scene factories used
//! across all integration test files.

#![allow(dead_code)]

use glam::{Vec2, Vec3, Vec4};
use solid_rs::prelude::*;
use std::io::{Cursor, Read, Seek, Write};

// ── Format descriptors ────────────────────────────────────────────────────────

pub static MOCK_FMT: FormatInfo = FormatInfo {
    name: "Mock Format",
    id: "mock",
    extensions: &["mock"],
    mime_types: &["model/x-mock"],
    can_load: true,
    can_save: true,
    spec_version: Some("1.0"),
};

pub static ALT_FMT: FormatInfo = FormatInfo {
    name: "Alt Format",
    id: "alt",
    extensions: &["alt"],
    mime_types: &["model/x-alt"],
    can_load: true,
    can_save: false,
    spec_version: None,
};

pub static SAVE_ONLY_FMT: FormatInfo = FormatInfo {
    name: "Save-Only Format",
    id: "sonly",
    extensions: &["sonly"],
    mime_types: &["model/x-sonly"],
    can_load: false,
    can_save: true,
    spec_version: None,
};

// ── Mock Loader ───────────────────────────────────────────────────────────────

/// Always succeeds; returns a one-triangle scene.
pub struct MockLoader;

impl Loader for MockLoader {
    fn load(&self, _r: &mut dyn ReadSeek, _o: &LoadOptions) -> Result<Scene> {
        Ok(make_triangle_scene())
    }
    fn format_info(&self) -> &FormatInfo {
        &MOCK_FMT
    }
    fn detect(&self, r: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 4];
        if r.read_exact(&mut buf).is_ok() && &buf == b"MOCK" {
            1.0
        } else {
            0.0
        }
    }
}

// ── Mock Saver ────────────────────────────────────────────────────────────────

/// Always succeeds; writes nothing.
pub struct MockSaver;

impl Saver for MockSaver {
    fn save(&self, _s: &Scene, _w: &mut dyn Write, _o: &SaveOptions) -> Result<()> {
        Ok(())
    }
    fn format_info(&self) -> &FormatInfo {
        &MOCK_FMT
    }
}

// ── Failing implementations ───────────────────────────────────────────────────

pub struct FailLoader;
impl Loader for FailLoader {
    fn load(&self, _r: &mut dyn ReadSeek, _o: &LoadOptions) -> Result<Scene> {
        Err(SolidError::parse("intentional failure"))
    }
    fn format_info(&self) -> &FormatInfo {
        &ALT_FMT
    }
}

pub struct FailSaver;
impl Saver for FailSaver {
    fn save(&self, _s: &Scene, _w: &mut dyn Write, _o: &SaveOptions) -> Result<()> {
        Err(SolidError::other("intentional failure"))
    }
    fn format_info(&self) -> &FormatInfo {
        &SAVE_ONLY_FMT
    }
}

// ── XYZ round-trip codec ──────────────────────────────────────────────────────
// A trivial text format: one "x y z" triplet per line.

pub static XYZ_FMT: FormatInfo = FormatInfo {
    name: "XYZ Point Cloud",
    id: "xyz",
    extensions: &["xyz"],
    mime_types: &["text/plain"],
    can_load: true,
    can_save: true,
    spec_version: None,
};

pub struct XyzLoader;
impl Loader for XyzLoader {
    fn load(&self, reader: &mut dyn ReadSeek, _o: &LoadOptions) -> Result<Scene> {
        use std::io::BufRead;
        let mut b = SceneBuilder::named("XYZ Scene");
        let mut mesh = Mesh::new("Points");
        for (i, line) in std::io::BufReader::new(reader).lines().enumerate() {
            let line = line.map_err(SolidError::Io)?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let v: Vec<f32> = line
                .split_whitespace()
                .map(|s| {
                    s.parse::<f32>()
                        .map_err(|_| SolidError::parse(format!("bad float line {}", i + 1)))
                })
                .collect::<Result<_>>()?;
            if v.len() < 3 {
                return Err(SolidError::parse(format!(
                    "need 3 coords on line {}",
                    i + 1
                )));
            }
            mesh.vertices.push(Vertex::new(Vec3::new(v[0], v[1], v[2])));
        }
        let n = mesh.vertices.len() as u32;
        mesh.primitives = vec![Primitive::points((0..n).collect(), None)];
        mesh.compute_bounds();
        let mi = b.push_mesh(mesh);
        let r = b.add_root_node("Root");
        b.attach_mesh(r, mi);
        Ok(b.build())
    }
    fn format_info(&self) -> &FormatInfo {
        &XYZ_FMT
    }
}

pub struct XyzSaver;
impl Saver for XyzSaver {
    fn save(&self, scene: &Scene, w: &mut dyn Write, opts: &SaveOptions) -> Result<()> {
        if let Some(c) = &opts.copyright {
            writeln!(w, "# {c}").map_err(SolidError::Io)?;
        }
        for mesh in &scene.meshes {
            for v in &mesh.vertices {
                let p = v.position;
                writeln!(w, "{} {} {}", p.x, p.y, p.z).map_err(SolidError::Io)?;
            }
        }
        Ok(())
    }
    fn format_info(&self) -> &FormatInfo {
        &XYZ_FMT
    }
}

// ── Counting visitor ──────────────────────────────────────────────────────────

#[derive(Default)]
pub struct CountingVisitor {
    pub nodes: usize,
    pub meshes: usize,
    pub materials: usize,
    pub textures: usize,
    pub images: usize,
    pub cameras: usize,
    pub lights: usize,
    pub skins: usize,
    pub animations: usize,
}

impl SceneVisitor for CountingVisitor {
    fn visit_node(&mut self, _: &Node) -> Result<()> {
        self.nodes += 1;
        Ok(())
    }
    fn visit_mesh(&mut self, _: &Mesh, _: usize) -> Result<()> {
        self.meshes += 1;
        Ok(())
    }
    fn visit_material(&mut self, _: &Material, _: usize) -> Result<()> {
        self.materials += 1;
        Ok(())
    }
    fn visit_texture(&mut self, _: &Texture, _: usize) -> Result<()> {
        self.textures += 1;
        Ok(())
    }
    fn visit_image(&mut self, _: &Image, _: usize) -> Result<()> {
        self.images += 1;
        Ok(())
    }
    fn visit_camera(&mut self, _: &Camera, _: usize) -> Result<()> {
        self.cameras += 1;
        Ok(())
    }
    fn visit_light(&mut self, _: &Light, _: usize) -> Result<()> {
        self.lights += 1;
        Ok(())
    }
    fn visit_skin(&mut self, _: &Skin, _: usize) -> Result<()> {
        self.skins += 1;
        Ok(())
    }
    fn visit_animation(&mut self, _: &Animation, _: usize) -> Result<()> {
        self.animations += 1;
        Ok(())
    }
}

// ── Error-triggering visitor ──────────────────────────────────────────────────

pub struct ErrorOnMesh;
impl SceneVisitor for ErrorOnMesh {
    fn visit_mesh(&mut self, _: &Mesh, _: usize) -> Result<()> {
        Err(SolidError::other("mesh visit error"))
    }
}

// ── Scene factories ───────────────────────────────────────────────────────────

pub fn make_empty_scene() -> Scene {
    Scene::new()
}

pub fn make_triangle_scene() -> Scene {
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

pub fn make_quad_scene() -> Scene {
    let mut b = SceneBuilder::named("Quad Scene");
    let mat = Material::solid_color("Red", Vec4::new(0.8, 0.1, 0.1, 1.0));
    let mi_m = b.push_material(mat);
    let mut mesh = Mesh::new("Quad");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)).with_uv(Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)).with_uv(Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, 1.0, 0.0)).with_uv(Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-1.0, 1.0, 0.0)).with_uv(Vec2::new(0.0, 1.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2, 0, 2, 3], Some(mi_m))];
    mesh.compute_bounds();
    let mi = b.push_mesh(mesh);
    let r = b.add_root_node("Root");
    b.attach_mesh(r, mi);
    b.build()
}

pub fn make_deep_hierarchy_scene(depth: usize) -> Scene {
    let mut b = SceneBuilder::named("Deep Hierarchy");
    let root = b.add_root_node("Root");
    let mut parent = root;
    for i in 0..depth {
        parent = b.add_child_node(parent, format!("Node_{i}"));
    }
    b.build()
}

pub fn make_full_scene() -> Scene {
    use glam::{Quat, Vec3};
    use std::f32::consts::PI;
    let mut b = SceneBuilder::named("Full Scene");

    let img_idx = b.push_image(Image::from_uri("img", "tex.png"));
    let tex_idx = b.push_texture(Texture::new("AlbedoTex", img_idx));
    let mut mat = Material::new("PBR");
    mat.base_color_texture = Some(TextureRef::new(tex_idx));
    let mat_idx = b.push_material(mat);

    let mut mesh = Mesh::new("Cube");
    mesh.vertices = (0..8)
        .map(|i| {
            Vertex::new(Vec3::new(
                if i & 1 != 0 { 1.0 } else { -1.0 },
                if i & 2 != 0 { 1.0 } else { -1.0 },
                if i & 4 != 0 { 1.0 } else { -1.0 },
            ))
        })
        .collect();
    mesh.primitives = vec![Primitive::triangles(
        vec![0, 1, 2, 1, 3, 2, 4, 6, 5, 5, 6, 7],
        Some(mat_idx),
    )];
    mesh.compute_bounds();
    let mesh_idx = b.push_mesh(mesh);

    let cam_idx = b.push_camera(Camera::perspective("MainCam"));
    let light_idx = b.push_light(Light::Directional(DirectionalLight {
        base: LightBase {
            name: "Sun".into(),
            color: Vec3::ONE,
            intensity: 2.0,
        },
        extensions: Extensions::new(),
    }));
    let mut anim = Animation::new("Rotate");
    anim.channels.push(AnimationChannel {
        target: AnimationTarget::Translation(NodeId(0)),
        interpolation: Interpolation::Linear,
        times: vec![0.0, 1.0],
        values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
    });
    b.push_animation(anim);

    let root = b.add_root_node("World");
    let mnode = b.add_child_node(root, "CubeNode");
    b.attach_mesh(mnode, mesh_idx);
    let cnode = b.add_child_node(root, "CamNode");
    b.set_transform(
        cnode,
        Transform::IDENTITY.with_translation(Vec3::new(0.0, 0.0, 5.0)),
    );
    b.attach_camera(cnode, cam_idx);
    let lnode = b.add_child_node(root, "LightNode");
    b.set_transform(
        lnode,
        Transform::IDENTITY.with_rotation(Quat::from_rotation_x(-PI / 4.0)),
    );
    b.attach_light(lnode, light_idx);

    b.build()
}

/// Encode a scene to an XYZ byte buffer and reload it.
pub fn xyz_round_trip(scene: &Scene) -> Scene {
    let mut buf = Vec::new();
    XyzSaver
        .save(scene, &mut buf, &SaveOptions::default())
        .unwrap();
    XyzLoader
        .load(&mut std::io::Cursor::new(buf), &LoadOptions::default())
        .unwrap()
}

/// Returns a simple triangle mesh (not yet in a scene).
pub fn make_triangle_mesh() -> Mesh {
    let mut mesh = Mesh::new("Triangle");
    mesh.vertices = vec![
        Vertex::new(Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(-1.0, -1.0, 0.0)),
        Vertex::new(Vec3::new(1.0, -1.0, 0.0)),
    ];
    mesh.primitives = vec![Primitive::triangles(vec![0, 1, 2], None)];
    mesh
}
