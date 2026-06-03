//! Axis-aligned bounding box (AABB).

use glam::Vec3;

/// A 3-D axis-aligned bounding box defined by its minimum and maximum corners.
#[derive(Debug, Clone, PartialEq)]
pub struct Aabb {
    /// The corner with the smallest x, y, and z coordinates.
    pub min: Vec3,
    /// The corner with the largest x, y, and z coordinates.
    pub max: Vec3,
}

impl Aabb {
    /// Creates an [`Aabb`] from explicit min/max corners.
    #[inline]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Computes the tightest [`Aabb`] enclosing `points`.
    /// Returns `None` if the iterator is empty.
    pub fn from_points(points: impl Iterator<Item = Vec3>) -> Option<Self> {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        let mut any = false;
        for p in points {
            min = min.min(p);
            max = max.max(p);
            any = true;
        }
        any.then_some(Self { min, max })
    }

    /// Returns the centre point of the box.
    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Returns the per-axis dimensions (width, height, depth).
    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Returns the half-extents (half the size along each axis).
    #[inline]
    pub fn half_extents(&self) -> Vec3 {
        self.size() * 0.5
    }

    /// Returns `true` if `point` is inside or on the surface of the box.
    #[inline]
    pub fn contains(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Returns the smallest [`Aabb`] that contains both `self` and `other`.
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns `true` if `self` and `other` overlap.
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }

    /// Returns the surface area of the box.
    pub fn surface_area(&self) -> f32 {
        let s = self.size();
        2.0 * (s.x * s.y + s.y * s.z + s.z * s.x)
    }

    /// Returns the volume of the box.
    pub fn volume(&self) -> f32 {
        let s = self.size();
        s.x * s.y * s.z
    }
}
