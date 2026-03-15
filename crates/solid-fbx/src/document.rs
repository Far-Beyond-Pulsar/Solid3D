//! Internal FBX document representation.
//!
//! An `FbxDocument` is the in-memory form produced by the binary or ASCII
//! parsers.  It is a direct reflection of the FBX node tree and is then
//! translated into a `solid_rs::Scene` by the [`crate::convert`] module.

// ── Document ─────────────────────────────────────────────────────────────────

/// An FBX document: a format version and the list of top-level nodes.
#[derive(Debug)]
pub(crate) struct FbxDocument {
    /// FBX version number, e.g. `7400` for FBX 7.4.
    pub version: u32,
    /// Top-level nodes (FBXHeaderExtension, Definitions, Objects, Connections …).
    pub roots: Vec<FbxNode>,
}

impl FbxDocument {
    /// Find the first top-level node whose name equals `name`.
    pub fn find(&self, name: &str) -> Option<&FbxNode> {
        self.roots.iter().find(|n| n.name == name)
    }
}

// ── Node ─────────────────────────────────────────────────────────────────────

/// A named FBX node with typed properties and optional child nodes.
#[derive(Debug, Clone)]
pub(crate) struct FbxNode {
    pub name: String,
    pub properties: Vec<FbxProperty>,
    pub children: Vec<FbxNode>,
}

impl FbxNode {
    /// Find the first direct child whose name equals `name`.
    #[inline]
    pub fn child(&self, name: &str) -> Option<&FbxNode> {
        self.children.iter().find(|n| n.name == name)
    }

    /// Iterate over every direct child whose name equals `name`.
    pub fn children_named(&self, name: &str) -> impl Iterator<Item = &FbxNode> {
        self.children.iter().filter(move |n| n.name == name)
    }

    // ── Convenience property accessors ───────────────────────────────────────

    /// First property as `i64` — used for FBX object IDs.
    pub fn id(&self) -> Option<i64> {
        self.properties.first().and_then(FbxProperty::as_i64)
    }

    /// Second property as `&str` — the FBX object name (e.g. `"Model::Box"`).
    pub fn object_name(&self) -> Option<&str> {
        self.properties.get(1).and_then(FbxProperty::as_str)
    }

    /// Third property as `&str` — the FBX object class (e.g. `"Mesh"`, `"Null"`).
    pub fn object_class(&self) -> Option<&str> {
        self.properties.get(2).and_then(FbxProperty::as_str)
    }

    /// First property as `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        self.properties.first().and_then(FbxProperty::as_f64)
    }

    /// First property as `&str`.
    pub fn as_str(&self) -> Option<&str> {
        self.properties.first().and_then(FbxProperty::as_str)
    }

    /// First property as a `f64` slice (for array-typed properties).
    pub fn as_f64_slice(&self) -> Option<&[f64]> {
        self.properties.first().and_then(FbxProperty::as_f64_slice)
    }

    /// First property as an `i32` slice.
    pub fn as_i32_slice(&self) -> Option<&[i32]> {
        self.properties.first().and_then(FbxProperty::as_i32_slice)
    }
}

// ── Property ──────────────────────────────────────────────────────────────────

/// A typed value attached to an [`FbxNode`].
#[derive(Debug, Clone)]
pub(crate) enum FbxProperty {
    // ── Scalar types (FBX type codes) ────────────────────────────────────────
    Bool(bool),         // 'C'
    Int16(i16),         // 'Y'
    Int32(i32),         // 'I'
    Int64(i64),         // 'L'
    Float32(f32),       // 'F'
    Float64(f64),       // 'D'
    // ── Array types ──────────────────────────────────────────────────────────
    ArrBool(Vec<bool>),    // 'b'
    ArrInt32(Vec<i32>),    // 'i'
    ArrInt64(Vec<i64>),    // 'l'
    ArrFloat32(Vec<f32>),  // 'f'
    ArrFloat64(Vec<f64>),  // 'd'
    // ── Blob types ───────────────────────────────────────────────────────────
    String(String),     // 'S'
    Bytes(Vec<u8>),     // 'R'
}

impl FbxProperty {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            FbxProperty::Int64(v)   => Some(*v),
            FbxProperty::Int32(v)   => Some(*v as i64),
            FbxProperty::Int16(v)   => Some(*v as i64),
            FbxProperty::Bool(v)    => Some(*v as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            FbxProperty::Float64(v) => Some(*v),
            FbxProperty::Float32(v) => Some(*v as f64),
            FbxProperty::Int64(v)   => Some(*v as f64),
            FbxProperty::Int32(v)   => Some(*v as f64),
            FbxProperty::Int16(v)   => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            FbxProperty::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64_slice(&self) -> Option<&[f64]> {
        match self {
            FbxProperty::ArrFloat64(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_f32_slice(&self) -> Option<&[f32]> {
        match self {
            FbxProperty::ArrFloat32(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_i32_slice(&self) -> Option<&[i32]> {
        match self {
            FbxProperty::ArrInt32(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_i64_slice(&self) -> Option<&[i64]> {
        match self {
            FbxProperty::ArrInt64(v) => Some(v),
            _ => None,
        }
    }
}
