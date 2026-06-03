mod common;
use common::*;
use solid_rs::prelude::*;

// pull the static into a convenient alias
use common::MOCK_FMT as MOCK_FORMAT_INFO;

// ── FormatInfo field access ───────────────────────────────────────────────────

#[test]
fn format_info_name() {
    assert_eq!(MOCK_FORMAT_INFO.name, "Mock Format");
}
#[test]
fn format_info_id() {
    assert_eq!(MOCK_FORMAT_INFO.id, "mock");
}
#[test]
fn format_info_can_load_true() {
    assert!(MOCK_FORMAT_INFO.can_load);
}
#[test]
fn format_info_can_save_true() {
    assert!(MOCK_FORMAT_INFO.can_save);
}
#[test]
fn format_info_spec_version_some() {
    assert!(MOCK_FORMAT_INFO.spec_version.is_some());
}

#[test]
fn format_info_load_only() {
    let info = FormatInfo {
        name: "Load Only",
        id: "lo",
        extensions: &["lo"],
        mime_types: &[],
        can_load: true,
        can_save: false,
        spec_version: None,
    };
    assert!(info.can_load);
    assert!(!info.can_save);
}

#[test]
fn format_info_save_only() {
    let info = FormatInfo {
        name: "Save Only",
        id: "so",
        extensions: &["so"],
        mime_types: &[],
        can_load: false,
        can_save: true,
        spec_version: None,
    };
    assert!(!info.can_load);
    assert!(info.can_save);
}

// ── matches_extension ─────────────────────────────────────────────────────────

#[test]
fn matches_extension_exact() {
    assert!(MOCK_FORMAT_INFO.matches_extension("mock"));
}
#[test]
fn matches_extension_uppercase() {
    assert!(MOCK_FORMAT_INFO.matches_extension("MOCK"));
}
#[test]
fn matches_extension_mixed_case() {
    assert!(MOCK_FORMAT_INFO.matches_extension("Mock"));
}
#[test]
fn matches_extension_no_match() {
    assert!(!MOCK_FORMAT_INFO.matches_extension("obj"));
}
#[test]
fn matches_extension_empty() {
    assert!(!MOCK_FORMAT_INFO.matches_extension(""));
}

#[test]
fn matches_extension_multiple_exts() {
    let info = FormatInfo {
        name: "Multi",
        id: "multi",
        extensions: &["a", "b", "c"],
        mime_types: &[],
        can_load: true,
        can_save: true,
        spec_version: None,
    };
    assert!(info.matches_extension("a"));
    assert!(info.matches_extension("b"));
    assert!(info.matches_extension("c"));
    assert!(!info.matches_extension("d"));
}

// ── matches_mime ──────────────────────────────────────────────────────────────

#[test]
fn matches_mime_exact() {
    assert!(MOCK_FORMAT_INFO.matches_mime("model/x-mock"));
}
#[test]
fn matches_mime_uppercase() {
    assert!(MOCK_FORMAT_INFO.matches_mime("MODEL/X-MOCK"));
}
#[test]
fn matches_mime_no_match() {
    assert!(!MOCK_FORMAT_INFO.matches_mime("model/obj"));
}
#[test]
fn matches_mime_empty() {
    assert!(!MOCK_FORMAT_INFO.matches_mime(""));
}

#[test]
fn matches_mime_multiple() {
    let info = FormatInfo {
        name: "X",
        id: "x",
        extensions: &[],
        mime_types: &["model/x-type1", "model/x-type2"],
        can_load: true,
        can_save: false,
        spec_version: None,
    };
    assert!(info.matches_mime("model/x-type1"));
    assert!(info.matches_mime("model/x-type2"));
    assert!(!info.matches_mime("model/x-type3"));
}

// ── spec_version ─────────────────────────────────────────────────────────────

#[test]
fn format_info_no_spec_version() {
    let info = FormatInfo {
        name: "X",
        id: "x",
        extensions: &[],
        mime_types: &[],
        can_load: false,
        can_save: false,
        spec_version: None,
    };
    assert!(info.spec_version.is_none());
}

#[test]
fn format_info_with_spec_version() {
    let info = FormatInfo {
        name: "X",
        id: "x",
        extensions: &[],
        mime_types: &[],
        can_load: false,
        can_save: false,
        spec_version: Some("2.0"),
    };
    assert_eq!(info.spec_version, Some("2.0"));
}

// ── empty extensions / mimes ─────────────────────────────────────────────────

#[test]
fn format_info_no_extensions() {
    let info = FormatInfo {
        name: "X",
        id: "x",
        extensions: &[],
        mime_types: &["model/x"],
        can_load: false,
        can_save: false,
        spec_version: None,
    };
    assert!(!info.matches_extension("x"));
    assert!(info.matches_mime("model/x"));
}

// ── Copy / Clone ─────────────────────────────────────────────────────────────

#[test]
fn format_info_copy() {
    let info = MOCK_FORMAT_INFO;
    let info2 = info;
    assert_eq!(info.id, info2.id);
}
