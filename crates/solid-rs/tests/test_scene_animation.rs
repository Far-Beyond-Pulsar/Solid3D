mod common;
use solid_rs::prelude::*;

// ── Interpolation ─────────────────────────────────────────────────────────────

#[test]
fn interpolation_default_is_linear() {
    assert_eq!(Interpolation::default(), Interpolation::Linear);
}
#[test]
fn interpolation_step_copy() {
    let a = Interpolation::Step;
    let b = a;
    assert_eq!(a, b);
}
#[test]
fn interpolation_cubic_distinct() {
    assert_ne!(Interpolation::CubicSpline, Interpolation::Linear);
}

// ── AnimationTarget ───────────────────────────────────────────────────────────

#[test]
fn animation_target_translation() {
    let t = AnimationTarget::Translation(NodeId(0));
    assert_eq!(t, AnimationTarget::Translation(NodeId(0)));
}

#[test]
fn animation_target_rotation() {
    let t = AnimationTarget::Rotation(NodeId(1));
    assert_eq!(t, AnimationTarget::Rotation(NodeId(1)));
}

#[test]
fn animation_target_scale() {
    let t = AnimationTarget::Scale(NodeId(2));
    assert_eq!(t, AnimationTarget::Scale(NodeId(2)));
}

#[test]
fn animation_target_morph_weight() {
    let t = AnimationTarget::MorphWeight {
        node_id: NodeId(3),
        target_index: 1,
    };
    assert_eq!(
        t,
        AnimationTarget::MorphWeight {
            node_id: NodeId(3),
            target_index: 1
        }
    );
}

#[test]
fn animation_target_morph_weight_different_index_not_eq() {
    let a = AnimationTarget::MorphWeight {
        node_id: NodeId(0),
        target_index: 0,
    };
    let b = AnimationTarget::MorphWeight {
        node_id: NodeId(0),
        target_index: 1,
    };
    assert_ne!(a, b);
}

// ── AnimationChannel ─────────────────────────────────────────────────────────

fn make_channel(times: Vec<f32>, values: Vec<f32>) -> AnimationChannel {
    AnimationChannel {
        target: AnimationTarget::Translation(NodeId(0)),
        interpolation: Interpolation::Linear,
        times,
        values,
    }
}

#[test]
fn channel_keyframe_count_zero() {
    assert_eq!(make_channel(vec![], vec![]).keyframe_count(), 0);
}
#[test]
fn channel_keyframe_count_three() {
    assert_eq!(
        make_channel(vec![0.0, 1.0, 2.0], vec![]).keyframe_count(),
        3
    );
}
#[test]
fn channel_duration_empty() {
    assert_eq!(make_channel(vec![], vec![]).duration(), 0.0);
}

#[test]
fn channel_duration_last_time() {
    assert_eq!(make_channel(vec![0.0, 0.5, 1.5], vec![]).duration(), 1.5);
}

#[test]
fn channel_duration_single_keyframe() {
    assert_eq!(make_channel(vec![3.7], vec![]).duration(), 3.7);
}

// ── Animation::new ────────────────────────────────────────────────────────────

#[test]
fn animation_new_sets_name() {
    let a = Animation::new("Walk");
    assert_eq!(a.name, "Walk");
}

#[test]
fn animation_new_channels_empty() {
    assert!(Animation::new("X").channels.is_empty());
}
#[test]
fn animation_new_extensions_empty() {
    assert!(Animation::new("X").extensions.is_empty());
}

// ── Animation::duration ──────────────────────────────────────────────────────

#[test]
fn animation_duration_no_channels() {
    assert_eq!(Animation::new("X").duration(), 0.0);
}

#[test]
fn animation_duration_single_channel() {
    let mut a = Animation::new("X");
    a.channels.push(make_channel(vec![0.0, 2.0], vec![]));
    assert_eq!(a.duration(), 2.0);
}

#[test]
fn animation_duration_max_across_channels() {
    let mut a = Animation::new("X");
    a.channels.push(make_channel(vec![0.0, 1.0], vec![]));
    a.channels.push(make_channel(vec![0.0, 3.0], vec![]));
    a.channels.push(make_channel(vec![0.0, 2.0], vec![]));
    assert_eq!(a.duration(), 3.0);
}

// ── Translation channel values ────────────────────────────────────────────────

#[test]
fn translation_channel_has_correct_value_count() {
    // 2 keyframes × 3 floats = 6 values
    let ch = AnimationChannel {
        target: AnimationTarget::Translation(NodeId(0)),
        interpolation: Interpolation::Linear,
        times: vec![0.0, 1.0],
        values: vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0],
    };
    assert_eq!(ch.values.len(), 6);
}

// ── Rotation channel values ───────────────────────────────────────────────────

#[test]
fn rotation_channel_quaternion_values() {
    // 1 keyframe × 4 floats (xyzw)
    let ch = AnimationChannel {
        target: AnimationTarget::Rotation(NodeId(0)),
        interpolation: Interpolation::Step,
        times: vec![0.0],
        values: vec![0.0, 0.0, 0.0, 1.0], // identity quat
    };
    assert_eq!(ch.values[3], 1.0); // w = 1
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn animation_clone_preserves_name() {
    let a = Animation::new("Jump");
    assert_eq!(a.clone().name, "Jump");
}

#[test]
fn animation_clone_preserves_channels_count() {
    let mut a = Animation::new("Run");
    a.channels.push(make_channel(vec![0.0, 1.0], vec![]));
    assert_eq!(a.clone().channels.len(), 1);
}

// ── Multiple targets ──────────────────────────────────────────────────────────

#[test]
fn animation_multiple_target_types() {
    let mut a = Animation::new("Complex");
    a.channels.push(AnimationChannel {
        target: AnimationTarget::Translation(NodeId(0)),
        interpolation: Interpolation::Linear,
        times: vec![0.0, 1.0],
        values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
    });
    a.channels.push(AnimationChannel {
        target: AnimationTarget::Rotation(NodeId(0)),
        interpolation: Interpolation::CubicSpline,
        times: vec![0.0, 1.0],
        values: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    });
    assert_eq!(a.channels.len(), 2);
    assert_eq!(a.duration(), 1.0);
}
