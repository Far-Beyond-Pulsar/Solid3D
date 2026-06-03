mod common;
use glam::Mat4;
use solid_rs::prelude::*;

// ── Skin::new ─────────────────────────────────────────────────────────────────

#[test]
fn skin_new_sets_name() {
    let s = Skin::new("ArmatureSkin");
    assert_eq!(s.name, "ArmatureSkin");
}

#[test]
fn skin_new_no_skeleton_root() {
    assert!(Skin::new("X").skeleton_root.is_none());
}
#[test]
fn skin_new_joints_empty() {
    assert!(Skin::new("X").joints.is_empty());
}
#[test]
fn skin_new_ibms_empty() {
    assert!(Skin::new("X").inverse_bind_matrices.is_empty());
}
#[test]
fn skin_new_extensions_empty() {
    assert!(Skin::new("X").extensions.is_empty());
}

// ── joint_count ──────────────────────────────────────────────────────────────

#[test]
fn joint_count_zero_initially() {
    assert_eq!(Skin::new("X").joint_count(), 0);
}

#[test]
fn joint_count_matches_joints_len() {
    let mut s = Skin::new("X");
    s.joints = vec![NodeId(0), NodeId(1), NodeId(2)];
    assert_eq!(s.joint_count(), 3);
}

// ── inverse_bind_matrix ───────────────────────────────────────────────────────

#[test]
fn ibm_out_of_range_returns_identity() {
    let s = Skin::new("X");
    assert_eq!(s.inverse_bind_matrix(0), Mat4::IDENTITY);
}

#[test]
fn ibm_in_range_returns_stored() {
    let mut s = Skin::new("X");
    let m = Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0));
    s.inverse_bind_matrices = vec![m];
    assert_eq!(s.inverse_bind_matrix(0), m);
}

#[test]
fn ibm_second_entry() {
    let mut s = Skin::new("X");
    let m0 = Mat4::IDENTITY;
    let m1 = Mat4::from_scale(glam::Vec3::splat(2.0));
    s.inverse_bind_matrices = vec![m0, m1];
    assert_eq!(s.inverse_bind_matrix(1), m1);
}

#[test]
fn ibm_index_beyond_vector_is_identity() {
    let mut s = Skin::new("X");
    s.inverse_bind_matrices = vec![Mat4::IDENTITY];
    assert_eq!(s.inverse_bind_matrix(99), Mat4::IDENTITY);
}

// ── skeleton_root ─────────────────────────────────────────────────────────────

#[test]
fn skin_skeleton_root_set() {
    let mut s = Skin::new("X");
    s.skeleton_root = Some(NodeId(5));
    assert_eq!(s.skeleton_root, Some(NodeId(5)));
}

// ── many joints ──────────────────────────────────────────────────────────────

#[test]
fn skin_many_joints() {
    let mut s = Skin::new("X");
    s.joints = (0..100).map(NodeId).collect();
    assert_eq!(s.joint_count(), 100);
}

// ── Clone ─────────────────────────────────────────────────────────────────────

#[test]
fn skin_clone_preserves_name() {
    let s = Skin::new("CloneSkin");
    assert_eq!(s.clone().name, "CloneSkin");
}

#[test]
fn skin_clone_preserves_joints() {
    let mut s = Skin::new("X");
    s.joints = vec![NodeId(0), NodeId(1)];
    let c = s.clone();
    assert_eq!(c.joints, vec![NodeId(0), NodeId(1)]);
}

// ── Extensions ────────────────────────────────────────────────────────────────

#[test]
fn skin_extensions_insert_and_get() {
    #[derive(Debug)]
    struct SkeletonMeta {
        version: u8,
    }
    let mut s = Skin::new("X");
    s.extensions.insert(SkeletonMeta { version: 2 });
    assert_eq!(s.extensions.get::<SkeletonMeta>().unwrap().version, 2);
}
