//! Regression tests for bugs filed against solid-usd:
//!   #27  USDZ textures are never extracted — materials get URI sources that
//!        point inside the zip and cannot be resolved.

use solid_rs::scene::ImageSource;
use solid_rs::prelude::*;
use solid_usd::UsdLoader;
use std::io::Cursor;

const TEXTURED_USDA: &str = r#"#usda 1.0
( defaultPrim = "Root" )

def Xform "Root" {
    def Material "Mat" {
        rel outputs:surface = </Root/Mat/Mat_Shader.outputs:surface>
        def Shader "Mat_Shader" {
            uniform token info:id = "UsdPreviewSurface"
            color3f inputs:diffuseColor = (0.8, 0.1, 0.1)
        }
        def Shader "Mat_Tex" {
            uniform token info:id = "UsdUVTexture"
            asset inputs:file = @Textures/albedo.png@
        }
    }
    def Mesh "Cube" {
        point3f[] points = [(0, 0, 0), (1, 0, 0), (1, 1, 0)]
        int[] faceVertexCounts = [3]
        int[] faceVertexIndices = [0, 1, 2]
        rel material:binding = </Root/Mat>
        uniform token subdivisionScheme = "none"
    }
}
"#;

fn make_usdz(usda: &str, assets: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default();
        w.start_file("root.usda", opts).unwrap();
        use std::io::Write;
        w.write_all(usda.as_bytes()).unwrap();
        for (name, data) in assets {
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }
    buf
}

fn load_usdz(bytes: Vec<u8>) -> Scene {
    UsdLoader
        .load(&mut Cursor::new(bytes), &LoadOptions::default())
        .expect("USDZ must load")
}

const PNG_BYTES: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x01, 0x02, 0x03];

#[test]
fn usdz_texture_is_extracted_as_embedded_bytes() {
    let bytes = make_usdz(TEXTURED_USDA, &[("Textures/albedo.png", PNG_BYTES)]);
    let scene = load_usdz(bytes);

    let img = scene
        .images
        .first()
        .expect("texture image must be present in the scene");
    match &img.source {
        ImageSource::Embedded { mime_type, data } => {
            assert_eq!(mime_type, "image/png");
            assert_eq!(data, PNG_BYTES);
        }
        other => panic!("expected embedded texture, got {other:?}"),
    }

    let mat = &scene.materials[0];
    let tr = mat
        .base_color_texture
        .as_ref()
        .expect("material must reference the texture");
    assert_eq!(tr.texture_index, 0);
}

#[test]
fn usdz_without_assets_falls_back_to_uri() {
    // No texture file in the archive: the reference stays a URI.
    let bytes = make_usdz(TEXTURED_USDA, &[]);
    let scene = load_usdz(bytes);
    let img = scene
        .images
        .first()
        .expect("texture image must be present in the scene");
    match &img.source {
        ImageSource::Uri(u) => assert_eq!(u, "Textures/albedo.png"),
        other => panic!("expected URI fallback, got {other:?}"),
    }
}

#[test]
fn usdz_embedded_asset_path_normalization() {
    // USD references `./Textures/albedo.png`; the zip entry is
    // `Textures/albedo.png`. Normalization must match them.
    let usda = TEXTURED_USDA.replace(
        "@Textures/albedo.png@",
        "@/Textures/albedo.png@",
    );
    let bytes = make_usdz(&usda, &[("Textures/albedo.png", PNG_BYTES)]);
    let scene = load_usdz(bytes);
    match &scene.images[0].source {
        ImageSource::Embedded { data, .. } => assert_eq!(data, PNG_BYTES),
        other => panic!("expected embedded texture after normalization, got {other:?}"),
    }
}
