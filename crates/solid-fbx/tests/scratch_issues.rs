//! Scratch tests to verify suspected parser/convert bugs against real-world
//! FBX features. TEMPORARY — will be removed once issues are filed/fixed.

use glam::Vec3;
use solid_fbx::FbxLoader;
use solid_rs::prelude::*;
use std::io::Cursor;

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

#[test]
fn control_point_normal_mapping() {
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
    // 8 expanded polygon-vertices.
    assert_eq!(mesh.vertices.len(), 8);
    // All corners reference a valid control point -> all normals must be (0,0,1).
    for (i, v) in mesh.vertices.iter().enumerate() {
        let n = v.normal.unwrap_or(Vec3::ZERO);
        assert!(
            (n - Vec3::Z).length() < 1e-5,
            "vertex {i}: normal {n:?} is wrong for ByControlPoint layer"
        );
    }
}

#[test]
fn uv_index_to_direct_mapping() {
    // UVs stored as a packed list, indexed per polygon-vertex via UVIndex
    // (this is what Blender emits). Must map correctly.
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
            ReferenceInformationType: "IndexToDirect"
            UV: *6 {{
                a: 0,0,1,0,0,1,1,1,0.5,0.5,0.25,0.25
            }}
            UVIndex: *8 {{
                a: 0,1,2,0,2,3,0,4
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
    // Corner 7 -> UVIndex 4 -> UV (0.5, 0.5); V is flipped for the engine.
    let uv7 = mesh.vertices[7].uvs[0].expect("UV should be set");
    assert!((uv7 - glam::Vec2::new(0.5, 0.5)).length() < 1e-5, "corner 7 uv {uv7:?}");
    // Corner 0 -> UVIndex 0 -> UV (0, 0) -> engine (0, 1).
    let uv0 = mesh.vertices[0].uvs[0].expect("UV should be set");
    assert!((uv0 - glam::Vec2::new(0.0, 1.0)).length() < 1e-5, "corner 0 uv {uv0:?}");
}

#[test]
fn secondary_uv_layer_is_imported() {
    // FBX files commonly carry a second UV set (lightmap / UV2). SolidRS
    // Vertex supports up to 8 UV channels, so channel 1 should be populated.
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
fn animation_keytime_as_floats() {
    // KeyTime array written with decimal points (real exporters do this).
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
