mod common;
use glam::{Vec2, Vec3, Vec4};
use solid_rs::prelude::*;

// ── Constants ─────────────────────────────────────────────────────────────────

#[test]
fn max_uv_channels_is_8() {
    assert_eq!(MAX_UV_CHANNELS, 8);
}
#[test]
fn max_color_channels_is_4() {
    assert_eq!(MAX_COLOR_CHANNELS, 4);
}

// ── Vertex::new ───────────────────────────────────────────────────────────────

#[test]
fn vertex_new_stores_position() {
    let v = Vertex::new(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(v.position, Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn vertex_new_negative_coords() {
    let v = Vertex::new(Vec3::new(-5.0, -3.0, -1.0));
    assert_eq!(v.position, Vec3::new(-5.0, -3.0, -1.0));
}

#[test]
fn vertex_new_zero_position() {
    let v = Vertex::new(Vec3::ZERO);
    assert_eq!(v.position, Vec3::ZERO);
}

#[test]
fn vertex_new_normal_initially_none() {
    assert!(Vertex::new(Vec3::ZERO).normal.is_none());
}
#[test]
fn vertex_new_tangent_initially_none() {
    assert!(Vertex::new(Vec3::ZERO).tangent.is_none());
}
#[test]
fn vertex_new_skin_weights_initially_none() {
    assert!(Vertex::new(Vec3::ZERO).skin_weights.is_none());
}

#[test]
fn vertex_new_uvs_all_initially_none() {
    let v = Vertex::new(Vec3::ZERO);
    for ch in 0..MAX_UV_CHANNELS {
        assert!(v.uvs[ch].is_none(), "channel {ch}");
    }
}

#[test]
fn vertex_new_colors_all_initially_none() {
    let v = Vertex::new(Vec3::ZERO);
    for ch in 0..MAX_COLOR_CHANNELS {
        assert!(v.colors[ch].is_none(), "channel {ch}");
    }
}

// ── Default ───────────────────────────────────────────────────────────────────

#[test]
fn vertex_default_position_is_zero() {
    assert_eq!(Vertex::default().position, Vec3::ZERO);
}

#[test]
fn vertex_default_equals_new_zero() {
    assert_eq!(Vertex::default(), Vertex::new(Vec3::ZERO));
}

// ── Builder methods ───────────────────────────────────────────────────────────

#[test]
fn with_normal_sets_normal() {
    let v = Vertex::new(Vec3::ZERO).with_normal(Vec3::Y);
    assert_eq!(v.normal, Some(Vec3::Y));
}

#[test]
fn with_normal_replaces_previous() {
    let v = Vertex::new(Vec3::ZERO)
        .with_normal(Vec3::X)
        .with_normal(Vec3::Y);
    assert_eq!(v.normal, Some(Vec3::Y));
}

#[test]
fn with_uv_sets_channel_0() {
    let uv = Vec2::new(0.5, 0.25);
    let v = Vertex::new(Vec3::ZERO).with_uv(uv);
    assert_eq!(v.uvs[0], Some(uv));
}

#[test]
fn with_uv_does_not_set_other_channels() {
    let v = Vertex::new(Vec3::ZERO).with_uv(Vec2::ONE);
    for ch in 1..MAX_UV_CHANNELS {
        assert!(v.uvs[ch].is_none(), "channel {ch} should be None");
    }
}

#[test]
fn with_color_sets_channel_0() {
    let c = Vec4::new(1.0, 0.0, 0.0, 1.0);
    let v = Vertex::new(Vec3::ZERO).with_color(c);
    assert_eq!(v.colors[0], Some(c));
}

#[test]
fn with_color_does_not_set_other_channels() {
    let v = Vertex::new(Vec3::ZERO).with_color(Vec4::ONE);
    for ch in 1..MAX_COLOR_CHANNELS {
        assert!(v.colors[ch].is_none(), "channel {ch}");
    }
}

#[test]
fn with_skin_weights_stores_weights() {
    let w = SkinWeights {
        joints: [0, 1, 2, 3],
        weights: [0.5, 0.3, 0.15, 0.05],
    };
    let v = Vertex::new(Vec3::ZERO).with_skin_weights(w.clone());
    let stored = v.skin_weights.as_ref().unwrap();
    assert_eq!(stored.joints, w.joints);
    assert_eq!(stored.weights, w.weights);
}

// ── Accessor shortcuts ────────────────────────────────────────────────────────

#[test]
fn uv_returns_channel_0() {
    let uv = Vec2::new(0.3, 0.7);
    let v = Vertex::new(Vec3::ZERO).with_uv(uv);
    assert_eq!(v.uv(), Some(uv));
}

#[test]
fn uv_returns_none_when_unset() {
    assert!(Vertex::new(Vec3::ZERO).uv().is_none());
}

#[test]
fn color_returns_channel_0() {
    let c = Vec4::new(0.2, 0.4, 0.6, 1.0);
    let v = Vertex::new(Vec3::ZERO).with_color(c);
    assert_eq!(v.color(), Some(c));
}

#[test]
fn color_returns_none_when_unset() {
    assert!(Vertex::new(Vec3::ZERO).color().is_none());
}

// ── Direct multi-channel access ───────────────────────────────────────────────

#[test]
fn set_uv_channel_1_directly() {
    let mut v = Vertex::new(Vec3::ZERO);
    v.uvs[1] = Some(Vec2::new(0.1, 0.9));
    assert!(v.uvs[0].is_none());
    assert_eq!(v.uvs[1], Some(Vec2::new(0.1, 0.9)));
}

#[test]
fn set_uv_channel_7_directly() {
    let mut v = Vertex::new(Vec3::ZERO);
    v.uvs[7] = Some(Vec2::new(0.5, 0.5));
    assert_eq!(v.uvs[7], Some(Vec2::new(0.5, 0.5)));
}

#[test]
fn set_color_channel_3_directly() {
    let mut v = Vertex::new(Vec3::ZERO);
    v.colors[3] = Some(Vec4::new(0.0, 1.0, 0.0, 0.5));
    assert!(v.colors[0].is_none());
    assert_eq!(v.colors[3], Some(Vec4::new(0.0, 1.0, 0.0, 0.5)));
}

#[test]
fn all_uv_channels_independently_set() {
    let mut v = Vertex::new(Vec3::ZERO);
    for ch in 0..MAX_UV_CHANNELS {
        v.uvs[ch] = Some(Vec2::new(ch as f32, ch as f32 * 0.1));
    }
    for ch in 0..MAX_UV_CHANNELS {
        assert_eq!(v.uvs[ch], Some(Vec2::new(ch as f32, ch as f32 * 0.1)));
    }
}

#[test]
fn all_color_channels_independently_set() {
    let mut v = Vertex::new(Vec3::ZERO);
    for ch in 0..MAX_COLOR_CHANNELS {
        v.colors[ch] = Some(Vec4::splat(ch as f32 * 0.25));
    }
    for ch in 0..MAX_COLOR_CHANNELS {
        assert_eq!(v.colors[ch], Some(Vec4::splat(ch as f32 * 0.25)));
    }
}

// ── Tangent ───────────────────────────────────────────────────────────────────

#[test]
fn set_tangent_with_handedness() {
    let mut v = Vertex::new(Vec3::ZERO);
    v.tangent = Some(Vec4::new(1.0, 0.0, 0.0, 1.0));
    let t = v.tangent.unwrap();
    assert_eq!(t.truncate(), Vec3::X);
    assert_eq!(t.w, 1.0); // right-handed
}

#[test]
fn set_tangent_left_handed() {
    let mut v = Vertex::new(Vec3::ZERO);
    v.tangent = Some(Vec4::new(1.0, 0.0, 0.0, -1.0));
    assert_eq!(v.tangent.unwrap().w, -1.0);
}

// ── SkinWeights ───────────────────────────────────────────────────────────────

#[test]
fn skin_weights_default_joints_are_zero() {
    assert_eq!(SkinWeights::default().joints, [0u16; 4]);
}

#[test]
fn skin_weights_default_weights_are_zero() {
    assert_eq!(SkinWeights::default().weights, [0.0f32; 4]);
}

#[test]
fn skin_weights_custom_joints_and_weights() {
    let w = SkinWeights {
        joints: [0, 2, 4, 6],
        weights: [0.5, 0.25, 0.15, 0.1],
    };
    assert_eq!(w.joints, [0, 2, 4, 6]);
    let sum: f32 = w.weights.iter().sum();
    assert!((sum - 1.0).abs() < 1e-5);
}

#[test]
fn skin_weights_clone() {
    let w = SkinWeights {
        joints: [1, 2, 3, 4],
        weights: [0.4, 0.3, 0.2, 0.1],
    };
    let c = w.clone();
    assert_eq!(c.joints, w.joints);
    assert_eq!(c.weights, w.weights);
}

// ── Clone & PartialEq ─────────────────────────────────────────────────────────

#[test]
fn vertex_clone_is_equal() {
    let v = Vertex::new(Vec3::new(1.0, 2.0, 3.0))
        .with_normal(Vec3::Y)
        .with_uv(Vec2::ONE);
    assert_eq!(v.clone(), v);
}

#[test]
fn vertex_partial_eq_same_position() {
    let v1 = Vertex::new(Vec3::new(1.0, 2.0, 3.0));
    let v2 = Vertex::new(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(v1, v2);
}

#[test]
fn vertex_partial_eq_different_position() {
    let v1 = Vertex::new(Vec3::new(1.0, 0.0, 0.0));
    let v2 = Vertex::new(Vec3::new(0.0, 1.0, 0.0));
    assert_ne!(v1, v2);
}

#[test]
fn vertex_partial_eq_different_normal() {
    let v1 = Vertex::new(Vec3::ZERO).with_normal(Vec3::X);
    let v2 = Vertex::new(Vec3::ZERO).with_normal(Vec3::Y);
    assert_ne!(v1, v2);
}

#[test]
fn vertex_partial_eq_different_uv() {
    let v1 = Vertex::new(Vec3::ZERO).with_uv(Vec2::new(0.0, 0.0));
    let v2 = Vertex::new(Vec3::ZERO).with_uv(Vec2::new(1.0, 1.0));
    assert_ne!(v1, v2);
}

// ── Builder chaining ──────────────────────────────────────────────────────────

#[test]
fn vertex_chain_all_builder_methods() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let normal = Vec3::Y;
    let uv = Vec2::new(0.5, 0.5);
    let color = Vec4::ONE;
    let w = SkinWeights {
        joints: [0, 1, 2, 3],
        weights: [0.25, 0.25, 0.25, 0.25],
    };

    let v = Vertex::new(pos)
        .with_normal(normal)
        .with_uv(uv)
        .with_color(color)
        .with_skin_weights(w);

    assert_eq!(v.position, pos);
    assert_eq!(v.normal, Some(normal));
    assert_eq!(v.uv(), Some(uv));
    assert_eq!(v.color(), Some(color));
    assert!(v.skin_weights.is_some());
}

#[test]
fn vertex_large_position_values() {
    let big = Vec3::new(1e30, -1e30, 1e30);
    let v = Vertex::new(big);
    assert_eq!(v.position, big);
}

#[test]
fn vertex_nan_position_stored_as_is() {
    let nan_v = Vertex::new(Vec3::new(f32::NAN, 0.0, 0.0));
    assert!(nan_v.position.x.is_nan());
}
