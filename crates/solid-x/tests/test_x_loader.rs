use solid_rs::prelude::*;
use solid_x::XLoader;
use std::io::Cursor;

#[test]
fn loads_ascii_x_mesh() {
    let x = b"xof 0303txt 0032
Mesh {
3;
0.0;0.0;0.0;,
1.0;0.0;0.0;,
0.0;1.0;0.0;;
1;
3;0,1,2;;
MeshNormals {
3;
0.0;0.0;1.0;,
0.0;0.0;1.0;,
0.0;0.0;1.0;;
1;
3;0,1,2;;
}
}";

    let scene = XLoader
        .load(&mut Cursor::new(&x[..]), &LoadOptions::default())
        .expect("failed to load ascii x");

    assert_eq!(scene.meshes.len(), 1);
    assert_eq!(scene.meshes[0].vertices.len(), 3);
    assert_eq!(scene.meshes[0].primitives[0].indices.len(), 3);
}

#[test]
fn rejects_binary_variant() {
    let x = b"xof 0303bin 0032";
    let err = XLoader
        .load(&mut Cursor::new(&x[..]), &LoadOptions::default())
        .expect_err("expected unsupported binary x");
    assert!(matches!(err, SolidError::UnsupportedFeature(_)));
}
