//! Regression tests for parser / converter bugs filed against solid-fbx.
//!
//! Each test maps to one GitHub issue:
//!   #2  ASCII FBX with a leading UTF-8 BOM is rejected
//!   #3  `ByControlPoint` layers index normals/UVs by the wrong index
//!   #4  `KeyTime` arrays written as floats drop whole animations
//!   #5  Binary save drops cameras, lights, skins and animations
//!   #6  Only the first UV / colour layer is imported
//!   #7  Binary parser pre-allocates up to GiB on hostile length fields
//!   #8  ASCII dashed identifiers / bare-word values (in-crate unit tests)
//!   #9  README vs lib.rs feature-table mismatch (docs)

mod common;
use common::*;

use glam::Vec3;
use solid_fbx::{FbxLoader, FbxSaver};
use solid_rs::prelude::*;
use std::io::Cursor;

const FBX_MAGIC: &[u8; 23] = b"Kaydara FBX Binary  \x00\x1a\x00";

fn load_ascii(src: &str) -> Scene {
    let mut cursor = Cursor::new(src.as_bytes().to_vec());
    FbxLoader
        .load(&mut cursor, &LoadOptions::default())
        .unwrap()
}

fn header() -> String {
    "; FBX 7.4.0 project file\nFBXHeaderExtension:  {\n    FBXHeaderVersion: 1003\n    FBXVersion: 7400\n}\nDefinitions:  {\n    Version: 100\n    Count: 3\n}\n".to_string()
}

fn close() -> String {
    "\nConnections:  {\n    C: \"OO\",100,200\n}\n".to_string()
}

// ── #2  UTF-8 BOM ─────────────────────────────────────────────────────────────

#[test]
fn bom_prefixed_ascii_fbx_loads() {
    let body = format!(
        r#"{header}
Objects:  {{
    Geometry: 100, "Geom", "Mesh"  {{
        Vertices: *12 {{
            a: 0,0,0,1,0,0,0,1,0,1,1,0
        }}
        PolygonVertexIndex: *8 {{
            a: 0,1,2,-4,2,3,0,-1
        }}
    }}
    Model: 200, "Model::Geom", "Mesh"  {{
        Version: 232
    }}
}}
{close}"#,
        header = header(),
        close = close()
    );

    let mut bytes = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    bytes.extend_from_slice(body.as_bytes());
    let mut cursor = Cursor::new(bytes);
    let scene = FbxLoader
        .load(&mut cursor, &LoadOptions::default())
        .expect("BOM-prefixed ASCII FBX should load");
    assert_eq!(scene.meshes.len(), 1);
}

// ── #3  ByControlPoint mapping ────────────────────────────────────────────────

#[test]
fn by_control_point_normals_map_to_corners() {
    // Two triangles sharing control points A,B,C,D.
    // Tri A = (0,1,2), Tri B = (2,3,0) -> pvi = 0,1,2,-4, 2,3,0,-1
    // Normals are per CONTROL POINT (4 entries) with Direct reference.
    let src = format!(
        r#"{header}
Objects:  {{
    Geometry: 100, "Geom", "Mesh"  {{
        Vertices: *12 {{
            a: 0,0,0,1,0,0,0,1,0,1,1,0
        }}
        PolygonVertexIndex: *8 {{
            a: 0,1,2,-4,2,3,0,-1
        }}
        LayerElementNormal: 0 {{
            Version: 101
            MappingInformationType: "ByControlPoint"
            ReferenceInformationType: "Direct"
            Normals: *12 {{
                a: 0,0,1,0,0,1,0,0,1,0,0,1
            }}
        }}
    }}
    Model: 200, "Model::Geom", "Mesh"  {{
        Version: 232
    }}
}}
{close}"#,
        header = header(),
        close = close()
    );

    let scene = load_ascii(&src);
    assert_eq!(scene.meshes.len(), 1, "mesh should be created");
    let mesh = &scene.meshes[0];
    assert_eq!(mesh.vertices.len(), 8);
    for (i, v) in mesh.vertices.iter().enumerate() {
        let n = v.normal.unwrap_or(Vec3::ZERO);
        assert!(
            (n - Vec3::Z).length() < 1e-5,
            "vertex {i}: normal {n:?} is wrong for ByControlPoint layer"
        );
    }
}

// ── #4  KeyTime floats ────────────────────────────────────────────────────────

#[test]
fn animation_keytime_as_floats_imports() {
    let src = format!(
        r#"{header}
Objects:  {{
    Model: 200, "Model::Geom", "Mesh"  {{
        Version: 232
    }}
    AnimationStack: 300, "Take 001", ""  {{
    }}
    AnimationLayer: 400, "BaseLayer", ""  {{
    }}
    AnimationCurveNode: 500, "AnimCurveNode::T", ""  {{
        Properties70:  {{
            P: "d|X", "Number", "", "A",0
            P: "d|Y", "Number", "", "A",0
            P: "d|Z", "Number", "", "A",0
        }}
    }}
    AnimationCurve: 600, "AnimCurve::", ""  {{
        Default: 0
        KeyTime: *2 {{
            a: 0.0,46186158000.0
        }}
        KeyValueFloat: *2 {{
            a: 0,1
        }}
    }}
    AnimationCurve: 601, "AnimCurve::", ""  {{
        Default: 0
        KeyTime: *2 {{
            a: 0.0,46186158000.0
        }}
        KeyValueFloat: *2 {{
            a: 0,2
        }}
    }}
    AnimationCurve: 602, "AnimCurve::", ""  {{
        Default: 0
        KeyTime: *2 {{
            a: 0.0,46186158000.0
        }}
        KeyValueFloat: *2 {{
            a: 0,3
        }}
    }}
}}
Connections:  {{
    C: "OP",600,500,"d|X"
    C: "OP",601,500,"d|Y"
    C: "OP",602,500,"d|Z"
    C: "OP",500,200,"Lcl Translation"
    C: "OO",500,400
    C: "OO",400,300
    C: "OO",300,0
}}
"#,
        header = header()
    );

    let scene = load_ascii(&src);
    assert_eq!(
        scene.animations.len(),
        1,
        "animation with float-typed KeyTimes should still be imported"
    );
    let anim = &scene.animations[0];
    assert_eq!(anim.channels.len(), 1);
    assert_eq!(anim.channels[0].times.len(), 2);
}

// ── #5  Binary save parity ────────────────────────────────────────────────────

#[test]
fn binary_round_trip_perspective_camera() {
    let scene = camera_scene(false);
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        loaded.cameras.len(),
        1,
        "camera lost in binary save ({} found)",
        loaded.cameras.len()
    );
}

#[test]
fn binary_round_trip_orthographic_camera() {
    let scene = camera_scene(true);
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        loaded.cameras.len(),
        1,
        "ortho camera lost in binary save ({} found)",
        loaded.cameras.len()
    );
}

#[test]
fn binary_round_trip_lights() {
    let scene = lights_scene();
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        loaded.lights.len(),
        4,
        "lights lost in binary save ({} found)",
        loaded.lights.len()
    );
}

#[test]
fn binary_round_trip_skins() {
    let scene = skinned_scene();
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        loaded.skins.len(),
        1,
        "skin lost in binary save ({} found)",
        loaded.skins.len()
    );
    assert!(
        !loaded.meshes.is_empty()
            && loaded.meshes[0]
                .vertices
                .iter()
                .any(|v| v.skin_weights.is_some()),
        "vertex skin weights lost in binary save"
    );
}

#[test]
fn binary_round_trip_animations() {
    let scene = animated_scene();
    let loaded = binary_round_trip(&scene);
    assert_eq!(
        loaded.animations.len(),
        1,
        "animation lost in binary save ({} found)",
        loaded.animations.len()
    );
    let anim = &loaded.animations[0];
    assert_eq!(
        anim.channels.len(),
        2,
        "animation channels lost in binary save"
    );
    assert_eq!(anim.channels[0].times.len(), 2, "keyframe times lost");
}

// ── #6  Secondary UV / colour layers ──────────────────────────────────────────

#[test]
fn secondary_uv_layer_is_imported() {
    let src = format!(
        r#"{header}
Objects:  {{
    Geometry: 100, "Geom", "Mesh"  {{
        Vertices: *12 {{
            a: 0,0,0,1,0,0,0,1,0,1,1,0
        }}
        PolygonVertexIndex: *8 {{
            a: 0,1,2,-4,2,3,0,-1
        }}
        LayerElementUV: 0 {{
            Version: 101
            Name: "UVMap"
            MappingInformationType: "ByPolygonVertex"
            ReferenceInformationType: "Direct"
            UV: *16 {{
                a: 0,0,1,0,0,1,1,1,0,0,1,0,0,1,1,1
            }}
        }}
        LayerElementUV: 1 {{
            Version: 101
            Name: "LightmapUV"
            MappingInformationType: "ByPolygonVertex"
            ReferenceInformationType: "Direct"
            UV: *16 {{
                a: 0.25,0.25,0.75,0.25,0.25,0.75,0.75,0.75,0.25,0.25,0.75,0.25,0.25,0.75,0.75,0.75
            }}
        }}
    }}
    Model: 200, "Model::Geom", "Mesh"  {{
        Version: 232
    }}
}}
{close}"#,
        header = header(),
        close = close()
    );

    let scene = load_ascii(&src);
    let mesh = &scene.meshes[0];
    assert_eq!(mesh.vertices.len(), 8);
    assert!(
        mesh.vertices[0].uvs[1].is_some(),
        "secondary UV layer should be imported into channel 1"
    );
}

#[test]
fn secondary_color_layer_is_imported() {
    let src = format!(
        r#"{header}
Objects:  {{
    Geometry: 100, "Geom", "Mesh"  {{
        Vertices: *12 {{
            a: 0,0,0,1,0,0,0,1,0,1,1,0
        }}
        PolygonVertexIndex: *8 {{
            a: 0,1,2,-4,2,3,0,-1
        }}
        LayerElementColor: 0 {{
            Version: 101
            MappingInformationType: "ByPolygonVertex"
            ReferenceInformationType: "Direct"
            Colors: *32 {{
                a: 1,0,0,1,0,1,0,1,0,0,1,1,1,1,0,1,1,0,0,1,0,1,0,1,0,0,1,1,1,1,0,1
            }}
        }}
        LayerElementColor: 1 {{
            Version: 101
            MappingInformationType: "ByPolygonVertex"
            ReferenceInformationType: "Direct"
            Colors: *32 {{
                a: 0.1,0.2,0.3,0.4,0.1,0.2,0.3,0.4,0.1,0.2,0.3,0.4,0.1,0.2,0.3,0.4,0.1,0.2,0.3,0.4,0.1,0.2,0.3,0.4,0.1,0.2,0.3,0.4,0.1,0.2,0.3,0.4
            }}
        }}
    }}
    Model: 200, "Model::Geom", "Mesh"  {{
        Version: 232
    }}
}}
{close}"#,
        header = header(),
        close = close()
    );

    let scene = load_ascii(&src);
    let mesh = &scene.meshes[0];
    assert_eq!(mesh.vertices.len(), 8);
    assert!(
        mesh.vertices[0].colors[0].is_some(),
        "primary color layer should be imported"
    );
    let c1 = mesh.vertices[0].colors[1].expect("secondary color layer should be imported");
    assert!(
        (c1.x - 0.1).abs() < 1e-5 && (c1.w - 0.4).abs() < 1e-5,
        "secondary color channel 1 value wrong: {c1:?}"
    );
}

// ── #7  Binary parser hostile length fields ───────────────────────────────────

fn malicious_node_fbx(prop_bytes: Vec<u8>) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(FBX_MAGIC);
    buf.extend_from_slice(&7400u32.to_le_bytes());
    let end_pos = buf.len();
    buf.extend_from_slice(&0u32.to_le_bytes()); // end_offset (patched below)
    buf.extend_from_slice(&1u32.to_le_bytes()); // num_props
    let prop_len_pos = buf.len();
    buf.extend_from_slice(&0u32.to_le_bytes()); // prop_list_len (patched)
    buf.push(4); // name length
    buf.extend_from_slice(b"Test");
    let props_start = buf.len();
    buf.extend_from_slice(&prop_bytes);
    let prop_list_len = (buf.len() - props_start) as u32;
    buf.extend_from_slice(&[0u8; 13]); // null sentinel
    let end_offset = buf.len() as u32;
    buf[end_pos..end_pos + 4].copy_from_slice(&end_offset.to_le_bytes());
    buf[prop_len_pos..prop_len_pos + 4].copy_from_slice(&prop_list_len.to_le_bytes());
    buf
}

fn huge_array_prop() -> Vec<u8> {
    let mut p = Vec::new();
    p.push(b'd'); // f64 array
    p.extend_from_slice(&u32::MAX.to_le_bytes()); // count -> ~34 GiB
    p.extend_from_slice(&0u32.to_le_bytes()); // encoding = raw
    p.extend_from_slice(&4u32.to_le_bytes()); // compressed_len
    p.extend_from_slice(&[0u8; 4]);
    p
}

fn huge_string_prop() -> Vec<u8> {
    let mut p = Vec::new();
    p.push(b'S'); // string
    p.extend_from_slice(&u32::MAX.to_le_bytes()); // length -> ~4 GiB
    p.extend_from_slice(b"tiny");
    p
}

#[test]
fn hostile_array_count_is_rejected_not_oomin() {
    let bytes = malicious_node_fbx(huge_array_prop());
    let mut cursor = Cursor::new(&bytes);
    let err = FbxLoader
        .load(&mut cursor, &LoadOptions::default())
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("array too large"),
        "expected size-guard error, got: {msg}"
    );
}

#[test]
fn hostile_string_length_is_rejected_not_oomin() {
    let bytes = malicious_node_fbx(huge_string_prop());
    let mut cursor = Cursor::new(&bytes);
    let err = FbxLoader
        .load(&mut cursor, &LoadOptions::default())
        .unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("unexpected end of file"),
        "expected EOF error, got: {msg}"
    );
}

// ── Round-trip sanity that the saver's binary output parses ───────────────────

#[test]
fn binary_save_round_trip_all_features_parses() {
    let mut b = SceneBuilder::new();
    let r = b.add_root_node("Root");
    let ci = b.push_camera(Camera::perspective("Cam"));
    let cn = b.add_child_node(r, "CamNode");
    b.attach_camera(cn, ci);
    let mi = b.push_mesh(make_minimal_mesh("M"));
    let mn = b.add_child_node(r, "MeshNode");
    b.attach_mesh(mn, mi);
    let scene = b.build();

    let mut buf = Vec::new();
    FbxSaver.save_binary(&scene, &mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let loaded = FbxLoader
        .load(&mut cursor, &LoadOptions::default())
        .expect("combined-feature binary scene must parse");
    assert_eq!(loaded.meshes.len(), 1);
    assert_eq!(loaded.cameras.len(), 1);
    assert_eq!(loaded.nodes.len(), scene.nodes.len());
}
