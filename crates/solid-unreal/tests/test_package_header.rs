use solid_unreal::{PackageIndex, UnrealLoader};
use solid_rs::traits::Loader;

#[test]
fn test_package_index() {
    let idx = PackageIndex(0);
    assert!(idx.is_null());
    assert!(!idx.is_export());
    assert!(!idx.is_import());

    let export = PackageIndex(5);
    assert!(!export.is_null());
    assert!(export.is_export());
    assert_eq!(export.to_index(), Some(4));

    let import = PackageIndex(-3);
    assert!(import.is_import());
    assert_eq!(import.to_index(), Some(2));
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
    // UE magic: 0x9E2A83C1 as big-endian bytes
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
