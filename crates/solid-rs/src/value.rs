//! Dynamically-typed [`Value`] for unstructured metadata.
//!
//! Used in [`Metadata::extra`](crate::scene::scene::Metadata::extra) and
//! wherever a format stores arbitrary key-value properties that do not map
//! cleanly onto a strongly-typed field.

use std::collections::HashMap;

/// A dynamically-typed value for storing unstructured metadata.
///
/// Covers the scalar and collection types found in most 3D formats' property
/// systems (FBX user properties, USD metadata, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Absence of a value.
    Null,
    /// Boolean flag.
    Bool(bool),
    /// Signed 64-bit integer.
    Int(i64),
    /// Double-precision float.
    Float(f64),
    /// UTF-8 string.
    String(String),
    /// 2-component float vector.
    Vec2([f32; 2]),
    /// 3-component float vector.
    Vec3([f32; 3]),
    /// 4-component float vector.
    Vec4([f32; 4]),
    /// Raw byte buffer (embedded binary blobs, compressed data, etc.).
    Bytes(Vec<u8>),
    /// Heterogeneous ordered list.
    Array(Vec<Value>),
    /// Heterogeneous string-keyed map.
    Map(HashMap<String, Value>),
}

impl Value {
    /// Returns the inner `bool`, or `None` if this is not a [`Value::Bool`].
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns the inner `i64`, or `None` if this is not a [`Value::Int`].
    pub fn as_int(&self) -> Option<i64> {
        if let Self::Int(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns the inner `f64`, or `None` if this is not a [`Value::Float`].
    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Returns the inner `&str`, or `None` if this is not a [`Value::String`].
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v.as_str())
        } else {
            None
        }
    }

    /// Returns the inner slice, or `None` if this is not a [`Value::Array`].
    pub fn as_array(&self) -> Option<&[Value]> {
        if let Self::Array(v) = self {
            Some(v.as_slice())
        } else {
            None
        }
    }

    /// Returns the inner map, or `None` if this is not a [`Value::Map`].
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        if let Self::Map(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` for [`Value::Null`].
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

// ── From impls ───────────────────────────────────────────────────────────────

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::Int(v as i64)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}
impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Self::Int(v as i64)
    }
}
impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Self::Float(v as f64)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::String(v.to_owned())
    }
}
impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Self::Bytes(v)
    }
}
impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Self::Array(v)
    }
}
impl From<HashMap<String, Value>> for Value {
    fn from(v: HashMap<String, Value>) -> Self {
        Self::Map(v)
    }
}
