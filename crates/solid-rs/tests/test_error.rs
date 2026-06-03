mod common;
use solid_rs::{Result, SolidError};
use std::io;

// ── Constructors ─────────────────────────────────────────────────────────────

#[test]
fn error_parse_constructor() {
    let e = SolidError::parse("line 5: unexpected EOF");
    assert!(matches!(e, SolidError::Parse(_)));
}

#[test]
fn error_parse_message_preserved() {
    let msg = "bad byte 0xFF at offset 12";
    let e = SolidError::parse(msg);
    if let SolidError::Parse(s) = e {
        assert_eq!(s, msg);
    } else {
        panic!();
    }
}

#[test]
fn error_unsupported_constructor() {
    let e = SolidError::unsupported("Draco compression");
    assert!(matches!(e, SolidError::UnsupportedFeature(_)));
}

#[test]
fn error_unsupported_message() {
    let e = SolidError::unsupported("feature X");
    if let SolidError::UnsupportedFeature(s) = e {
        assert_eq!(s, "feature X");
    } else {
        panic!();
    }
}

#[test]
fn error_format_constructor() {
    let e = SolidError::format("fbx", "malformed node header");
    assert!(matches!(e, SolidError::Format { .. }));
}

#[test]
fn error_format_fields() {
    let e = SolidError::format("obj", "missing 'v'");
    if let SolidError::Format { format, message } = e {
        assert_eq!(format, "obj");
        assert_eq!(message, "missing 'v'");
    } else {
        panic!();
    }
}

#[test]
fn error_invalid_ref_constructor() {
    let e = SolidError::invalid_ref("material index 5 out of bounds");
    assert!(matches!(e, SolidError::InvalidReference(_)));
}

#[test]
fn error_other_constructor() {
    let e = SolidError::other("unexpected internal state");
    assert!(matches!(e, SolidError::Other(_)));
}

// ── From<io::Error> ───────────────────────────────────────────────────────────

#[test]
fn error_from_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let e: SolidError = io_err.into();
    assert!(matches!(e, SolidError::Io(_)));
}

#[test]
fn error_io_question_mark() {
    fn fails() -> Result<()> {
        let _f = std::fs::File::open("/this/path/does/not/exist/ever")?;
        Ok(())
    }
    let r = fails();
    assert!(r.is_err());
    assert!(matches!(r.unwrap_err(), SolidError::Io(_)));
}

// ── Display ───────────────────────────────────────────────────────────────────

#[test]
fn error_display_parse() {
    let s = SolidError::parse("bad data").to_string();
    assert!(s.contains("bad data"));
}

#[test]
fn error_display_format_contains_format_name() {
    let s = SolidError::format("gltf", "invalid JSON").to_string();
    assert!(s.contains("gltf"));
    assert!(s.contains("invalid JSON"));
}

#[test]
fn error_display_unsupported() {
    let s = SolidError::unsupported("feature Y").to_string();
    assert!(s.contains("feature Y"));
}

#[test]
fn error_display_other() {
    let s = SolidError::other("oops").to_string();
    assert!(s.contains("oops"));
}

#[test]
fn error_display_io() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    let s = SolidError::from(io_err).to_string();
    assert!(s.contains("I/O") || s.contains("io") || s.contains("denied"));
}

// ── Result alias ─────────────────────────────────────────────────────────────

#[test]
fn result_ok_is_ok() {
    let r: Result<i32> = Ok(42);
    assert_eq!(r.unwrap(), 42);
}

#[test]
fn result_err_is_err() {
    let r: Result<i32> = Err(SolidError::parse("x"));
    assert!(r.is_err());
}

// ── InvalidScene / InvalidReference ──────────────────────────────────────────

#[test]
fn error_invalid_scene() {
    let e = SolidError::InvalidScene("cyclic hierarchy".into());
    assert!(matches!(e, SolidError::InvalidScene(_)));
    assert!(e.to_string().contains("cyclic"));
}

#[test]
fn error_invalid_reference_display() {
    let e = SolidError::invalid_ref("node 99 missing");
    assert!(e.to_string().contains("node 99"));
}

// ── UnsupportedFormat ─────────────────────────────────────────────────────────

#[test]
fn error_unsupported_format() {
    let e = SolidError::UnsupportedFormat("no loader for .xyz".into());
    assert!(matches!(e, SolidError::UnsupportedFormat(_)));
    assert!(e.to_string().contains(".xyz"));
}

// ── Error can be returned from fn ────────────────────────────────────────────

#[test]
fn error_in_result_chain() {
    fn load_data() -> Result<u32> {
        Err(SolidError::parse("data error"))
    }
    fn process() -> Result<String> {
        let _v = load_data()?; // ? propagation
        Ok("ok".into())
    }
    assert!(process().is_err());
}

// ── Debug ────────────────────────────────────────────────────────────────────

#[test]
fn error_debug_output() {
    let e = SolidError::parse("test");
    let s = format!("{e:?}");
    assert!(!s.is_empty());
}
