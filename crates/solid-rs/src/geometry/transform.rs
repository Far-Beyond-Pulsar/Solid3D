//! Node local transform: translation, rotation (quaternion), and scale.

use glam::{Mat4, Quat, Vec3};

/// Decomposed TRS (Translation · Rotation · Scale) transform stored on a
/// [`Node`](crate::scene::Node).
///
/// The final local matrix is `T * R * S` (applied right-to-left):
///
/// ```text
/// local_matrix = translation * rotation * scale
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    /// Position offset relative to the parent node.
    pub translation: Vec3,
    /// Orientation as a unit quaternion.
    pub rotation: Quat,
    /// Per-axis scale factors.
    pub scale: Vec3,
}

impl Transform {
    /// The identity transform: no translation, no rotation, scale = 1.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Decomposes an affine `Mat4` into a TRS transform.
    ///
    /// Non-affine matrices (e.g. with shear) lose the shear component.
    pub fn from_matrix(mat: Mat4) -> Self {
        let (scale, rotation, translation) = mat.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Recomposes this transform into a 4×4 matrix.
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Builder-style setter for translation.
    #[inline]
    pub fn with_translation(mut self, t: Vec3) -> Self {
        self.translation = t;
        self
    }

    /// Builder-style setter for rotation.
    #[inline]
    pub fn with_rotation(mut self, r: Quat) -> Self {
        self.rotation = r;
        self
    }

    /// Builder-style setter for scale.
    #[inline]
    pub fn with_scale(mut self, s: Vec3) -> Self {
        self.scale = s;
        self
    }

    /// Returns `true` if this transform is (approximately) the identity.
    pub fn is_identity(&self) -> bool {
        self.translation.abs_diff_eq(Vec3::ZERO, 1e-6)
            && self.rotation.abs_diff_eq(Quat::IDENTITY, 1e-6)
            && self.scale.abs_diff_eq(Vec3::ONE, 1e-6)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
