use std::io::Cursor;

use solid_gltf::GltfLoader;
use solid_rs::prelude::*;

fn load(document: &serde_json::Value) -> solid_rs::Result<solid_rs::Scene> {
    GltfLoader.load(
        &mut Cursor::new(serde_json::to_vec(document).unwrap()),
        &LoadOptions::default(),
    )
}

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/multi_skin.gltf")).unwrap()
}

#[test]
fn all_sets_are_ranked_before_normalization_and_ties_use_joint_index() {
    let scene = load(&fixture()).unwrap();
    let mesh = &scene.meshes[0];
    assert_eq!(mesh.vertices.len(), 6);
    assert_eq!(mesh.primitives.len(), 2);
    for primitive in mesh.vertices.as_chunks::<3>().0 {
        let vertex = primitive[0].skin_weights.as_ref().unwrap();
        assert_eq!(vertex.joints, [8, 6, 5, 4]);
        for (actual, expected) in vertex.weights.iter().zip([0.625, 0.15625, 0.125, 0.09375]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        let tied = primitive[1].skin_weights.as_ref().unwrap();
        assert_eq!(tied.joints, [0, 1, 2, 3]);
        assert_eq!(tied.weights, [0.25; 4]);
        let two = primitive[2].skin_weights.as_ref().unwrap();
        assert_eq!(two.joints, [7, 1, 0, 0]);
        assert_eq!(two.weights, [0.75, 0.25, 0.0, 0.0]);
    }
    assert_eq!(scene.skins.len(), 2);
    let instances: Vec<_> = scene
        .nodes
        .iter()
        .filter(|n| n.mesh == Some(0))
        .map(|n| n.skin)
        .collect();
    assert_eq!(instances, [Some(0), Some(1), Some(0), None]);
    assert_ne!(scene.skins[0].joints, scene.skins[1].joints);
    assert_eq!(scene.skins[1].inverse_bind_matrices[0].w_axis.x, -20.0);
}

#[test]
fn malformed_skin_attributes_are_errors() {
    for (attribute, field, value) in [
        ("JOINTS_0", "componentType", serde_json::json!(5125)),
        ("WEIGHTS_1", "count", serde_json::json!(2)),
        ("WEIGHTS_2", "type", serde_json::json!("VEC3")),
    ] {
        let mut document = fixture();
        let index = document["meshes"][0]["primitives"][0]["attributes"][attribute]
            .as_u64()
            .unwrap() as usize;
        document["accessors"][index][field] = value;
        assert!(load(&document).is_err(), "{attribute}.{field}");
    }
    let mut document = fixture();
    document["meshes"][0]["primitives"][0]["attributes"]
        .as_object_mut()
        .unwrap()
        .remove("WEIGHTS_1");
    assert!(load(&document).unwrap_err().to_string().contains("paired"));
    let mut document = fixture();
    document["skins"][1]["joints"] = serde_json::json!([3]);
    assert!(load(&document).unwrap_err().to_string().contains("palette"));
}

#[test]
fn reduction_warning_is_aggregated_per_mesh() {
    struct Logger(std::sync::Mutex<Vec<String>>);
    impl log::Log for Logger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() == log::Level::Warn
        }
        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) {
                self.0.lock().unwrap().push(record.args().to_string());
            }
        }
        fn flush(&self) {}
    }
    static LOGGER: Logger = Logger(std::sync::Mutex::new(Vec::new()));
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Warn);
    let mut document = fixture();
    document["meshes"][0]["name"] = serde_json::json!("WarningFixture");
    load(&document).unwrap();
    let messages = LOGGER.0.lock().unwrap();
    let warnings: Vec<_> = messages
        .iter()
        .filter(|message| message.contains("WarningFixture"))
        .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("4 vertices"));
    assert!(warnings[0].contains("strongest four"));
}

#[test]
fn invalid_and_duplicate_weights_are_cleaned_before_selection() {
    use base64::Engine;
    let mut document = fixture();
    let uri = document["buffers"][0]["uri"].as_str().unwrap();
    let mut bytes = base64::engine::general_purpose::STANDARD
        .decode(uri.split_once(',').unwrap().1)
        .unwrap();
    let attributes = &document["meshes"][0]["primitives"][0]["attributes"];
    for (set, weights) in [
        (
            0,
            [
                f32::NAN,
                -1.0,
                f32::INFINITY,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.1,
                0.2,
                0.3,
                0.4,
            ],
        ),
        (
            1,
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            2,
            [
                f32::MAX,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        ),
    ] {
        let accessor = attributes[format!("WEIGHTS_{set}")].as_u64().unwrap() as usize;
        let view = document["accessors"][accessor]["bufferView"]
            .as_u64()
            .unwrap() as usize;
        let start = document["bufferViews"][view]["byteOffset"]
            .as_u64()
            .unwrap() as usize;
        for (index, weight) in weights.into_iter().enumerate() {
            bytes[start + index * 4..start + index * 4 + 4].copy_from_slice(&weight.to_le_bytes());
        }
    }
    document["buffers"][0]["uri"] = serde_json::json!(format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ));
    let scene = load(&document).unwrap();
    let vertices = &scene.meshes[0].vertices;
    assert_eq!(
        vertices[0].skin_weights.as_ref().unwrap().weights,
        [1.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(vertices[1].skin_weights.as_ref().unwrap().weights, [0.0; 4]);
    let duplicated = vertices[2].skin_weights.as_ref().unwrap();
    assert_eq!(duplicated.joints, [0, 7, 1, 0]);
    assert!((duplicated.weights[0] - 0.7).abs() < 1e-6);
}

#[test]
fn non_default_scenes_keep_joint_palettes_and_shared_nodes_are_loaded_once() {
    let mut document = fixture();
    document["scenes"] = serde_json::json!([
        {"nodes": [0, 2, 20, 21]}, {"nodes": [1, 2]}
    ]);
    let scene = load(&document).unwrap();
    assert_eq!(scene.nodes.len(), 22);
    assert_eq!(scene.skins[1].joints.len(), 9);
    assert_eq!(scene.skins[0].joints[0], scene.skins[1].joints[8]);
    assert!(scene.nodes.iter().any(|n| n.skin == Some(1)));
}

#[test]
fn missing_inverse_binds_are_identity_and_broken_joint_tables_are_errors() {
    let mut document = fixture();
    document["skins"][1]
        .as_object_mut()
        .unwrap()
        .remove("inverseBindMatrices");
    let scene = load(&document).unwrap();
    assert_eq!(scene.skins[1].inverse_bind_matrix(8), glam::Mat4::IDENTITY);
    document["skins"][1]["joints"][0] = serde_json::json!(999);
    assert!(load(&document)
        .unwrap_err()
        .to_string()
        .contains("missing joint"));
    let mut document = fixture();
    document["nodes"][3]["children"] = serde_json::json!([2]);
    assert!(load(&document).is_err());
}
