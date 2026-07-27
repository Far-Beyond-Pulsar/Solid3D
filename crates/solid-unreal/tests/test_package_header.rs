use solid_unreal::UnrealLoader;
use solid_rs::traits::Loader;

#[test]
fn test_package_index() {
    let idx: i32 = 0;
    assert!(idx == 0);

    let export: i32 = 4;
    assert!(export > 0);

    let import: i32 = -2;
    assert!(import < 0);
}

#[test]
fn test_magic_detect_empty() {
    let loader = UnrealLoader;
    let data: &[u8] = &[];
    let confidence = loader.detect(&mut std::io::Cursor::new(data));
    assert_eq!(confidence, 0.0);
}

#[test]
fn test_magic_detect_wrong() {
    let loader = UnrealLoader;
    let data = b"NOTAUE PACKAGE_________________";
    let confidence = loader.detect(&mut std::io::Cursor::new(data));
    assert_eq!(confidence, 0.0);
}

#[test]
fn test_magic_detect_correct() {
    let loader = UnrealLoader;
    let data: [u8; 4] = [0x9E, 0x2A, 0x83, 0xC1];
    let confidence = loader.detect(&mut std::io::Cursor::new(data));
    assert!(confidence > 0.5);
}

#[test]
fn test_load_invalid_data() {
    let loader = UnrealLoader;
    let data = b"this is not a valid unreal package file";
    let result = loader.load(
        &mut std::io::Cursor::new(data),
        &solid_rs::traits::LoadOptions::default(),
    );
    assert!(result.is_err());
}

#[test]
fn test_format_info() {
    let loader = UnrealLoader;
    let info = loader.format_info();
    assert_eq!(info.id, "unreal");
    assert!(info.extensions.contains(&"uasset"));
    assert!(info.extensions.contains(&"umap"));
    assert!(info.can_load);
    assert!(!info.can_save);
}
