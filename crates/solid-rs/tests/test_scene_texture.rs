mod common;
use solid_rs::prelude::*;

// ── ImageSource ───────────────────────────────────────────────────────────────

#[test]
fn image_from_uri_sets_source() {
    let img = Image::from_uri("albedo", "textures/albedo.png");
    match &img.source {
        ImageSource::Uri(s) => assert_eq!(s, "textures/albedo.png"),
        _ => panic!("expected URI"),
    }
}

#[test]
fn image_from_uri_sets_name() {
    let img = Image::from_uri("MyImage", "path/to/img.png");
    assert_eq!(img.name, "MyImage");
}

#[test]
fn image_embedded_sets_source() {
    let data = vec![0u8, 1, 2, 3];
    let img = Image::embedded("bump", "image/png", data.clone());
    match &img.source {
        ImageSource::Embedded { mime_type, data: d } => {
            assert_eq!(mime_type, "image/png");
            assert_eq!(*d, data);
        }
        _ => panic!("expected Embedded"),
    }
}

#[test]
fn image_embedded_sets_name() {
    let img = Image::embedded("N", "image/jpeg", vec![]);
    assert_eq!(img.name, "N");
}

#[test]
fn image_extensions_initially_empty() {
    assert!(Image::from_uri("X", "y").extensions.is_empty());
}

// ── WrapMode ─────────────────────────────────────────────────────────────────

#[test]
fn wrap_mode_default_is_repeat() {
    assert_eq!(WrapMode::default(), WrapMode::Repeat);
}
#[test]
fn wrap_mode_copy() {
    let a = WrapMode::ClampToEdge;
    let b = a;
    assert_eq!(a, b);
}
#[test]
fn wrap_mode_eq_same() {
    assert_eq!(WrapMode::Repeat, WrapMode::Repeat);
}
#[test]
fn wrap_mode_ne_different() {
    assert_ne!(WrapMode::Repeat, WrapMode::ClampToEdge);
}
#[test]
fn wrap_mode_mirrored_distinct() {
    assert_ne!(WrapMode::MirroredRepeat, WrapMode::Repeat);
}

// ── FilterMode ────────────────────────────────────────────────────────────────

#[test]
fn filter_mode_default_is_trilinear() {
    assert_eq!(FilterMode::default(), FilterMode::LinearMipmapLinear);
}
#[test]
fn filter_mode_nearest_copy() {
    let a = FilterMode::Nearest;
    let b = a;
    assert_eq!(a, b);
}
#[test]
fn filter_mode_ne_different() {
    assert_ne!(FilterMode::Nearest, FilterMode::Linear);
}

// ── Sampler ───────────────────────────────────────────────────────────────────

#[test]
fn sampler_default_mag_filter() {
    let s = Sampler::default();
    assert_eq!(s.mag_filter, FilterMode::LinearMipmapLinear);
}

#[test]
fn sampler_default_wrap_repeat() {
    let s = Sampler::default();
    assert_eq!(s.wrap_s, WrapMode::Repeat);
    assert_eq!(s.wrap_t, WrapMode::Repeat);
}

// ── Texture::new ─────────────────────────────────────────────────────────────

#[test]
fn texture_new_sets_name() {
    let t = Texture::new("AlbedoTex", 0);
    assert_eq!(t.name, "AlbedoTex");
}

#[test]
fn texture_new_sets_image_index() {
    let t = Texture::new("X", 5);
    assert_eq!(t.image_index, 5);
}

#[test]
fn texture_new_default_sampler() {
    let t = Texture::new("X", 0);
    assert_eq!(t.sampler.wrap_s, WrapMode::Repeat);
}

#[test]
fn texture_extensions_empty() {
    assert!(Texture::new("X", 0).extensions.is_empty());
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn texture_clone_preserves_image_index() {
    let t = Texture::new("T", 7);
    assert_eq!(t.clone().image_index, 7);
}

#[test]
fn image_clone_uri() {
    let img = Image::from_uri("img", "foo/bar.png");
    let c = img.clone();
    match &c.source {
        ImageSource::Uri(s) => assert_eq!(s, "foo/bar.png"),
        _ => panic!(),
    }
}

// ── ImageSource equality ──────────────────────────────────────────────────────

#[test]
fn image_source_uri_eq() {
    let a = ImageSource::Uri("a.png".into());
    let b = ImageSource::Uri("a.png".into());
    assert_eq!(a, b);
}

#[test]
fn image_source_uri_ne() {
    let a = ImageSource::Uri("a.png".into());
    let b = ImageSource::Uri("b.png".into());
    assert_ne!(a, b);
}

#[test]
fn image_source_embedded_eq_same_data() {
    let a = ImageSource::Embedded {
        mime_type: "image/png".into(),
        data: vec![1, 2, 3],
    };
    let b = ImageSource::Embedded {
        mime_type: "image/png".into(),
        data: vec![1, 2, 3],
    };
    assert_eq!(a, b);
}

#[test]
fn image_source_embedded_ne_different_data() {
    let a = ImageSource::Embedded {
        mime_type: "image/png".into(),
        data: vec![1, 2, 3],
    };
    let b = ImageSource::Embedded {
        mime_type: "image/png".into(),
        data: vec![4, 5, 6],
    };
    assert_ne!(a, b);
}

// ── Custom sampler ────────────────────────────────────────────────────────────

#[test]
fn texture_custom_sampler() {
    let mut t = Texture::new("X", 0);
    t.sampler.wrap_s = WrapMode::ClampToEdge;
    t.sampler.wrap_t = WrapMode::MirroredRepeat;
    t.sampler.mag_filter = FilterMode::Nearest;
    assert_eq!(t.sampler.wrap_s, WrapMode::ClampToEdge);
    assert_eq!(t.sampler.wrap_t, WrapMode::MirroredRepeat);
    assert_eq!(t.sampler.mag_filter, FilterMode::Nearest);
}
