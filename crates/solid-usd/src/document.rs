//! In-memory representation of a USD document.
//!
//! This is a simplified but practical model that covers the subset of USD
//! understood by this crate's loader and saver:
//!
//! * `def` / `over` / `class` prims with typed specifiers (Xform, Mesh, …)
//! * Typed attributes (`point3f[]`, `float3`, `color3f`, `token`, …)
//! * Relationship declarations
//! * Stage-level metadata (`upAxis`, `metersPerUnit`, `defaultPrim`)

use std::collections::HashMap;

// ── Value types ──────────────────────────────────────────────────────────────

/// A scalar or array value stored on a USD attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum UsdValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Token(String),
    Asset(String),
    Vec2f([f64; 2]),
    Vec3f([f64; 3]),
    Vec4f([f64; 4]),
    Matrix4d([[f64; 4]; 4]),
    /// `int[]`
    IntArray(Vec<i64>),
    /// `float[]`
    FloatArray(Vec<f64>),
    /// `point3f[]` / `normal3f[]` / `vector3f[]` / `color3f[]`
    Vec3fArray(Vec<[f64; 3]>),
    /// `texCoord2f[]`
    Vec2fArray(Vec<[f64; 2]>),
    /// `token[]`
    TokenArray(Vec<String>),
    /// `string[]`
    StringArray(Vec<String>),
}

// ── Attribute / relationship ─────────────────────────────────────────────────

/// A single named attribute on a [`Prim`].
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Attribute name (e.g. `"points"`, `"extent"`).
    pub name: String,
    /// The declared USD type string (e.g. `"point3f[]"`, `"float"`).
    pub type_name: String,
    /// The stored value, if any.
    pub value: Option<UsdValue>,
    /// Whether the attribute is declared `uniform`.
    pub uniform: bool,
}

impl Attribute {
    pub fn new(name: impl Into<String>, type_name: impl Into<String>, value: UsdValue) -> Self {
        Self {
            name: name.into(),
            type_name: type_name.into(),
            value: Some(value),
            uniform: false,
        }
    }

    pub fn uniform(mut self) -> Self {
        self.uniform = true;
        self
    }
}

/// A relationship (e.g. `rel material:binding`).
#[derive(Debug, Clone)]
pub struct Relationship {
    /// Relationship name (e.g. `"material:binding"`).
    pub name: String,
    /// Target SdfPath (e.g. `"/World/Mat0"`).
    pub target: Option<String>,
}

// ── Prim ─────────────────────────────────────────────────────────────────────

/// How a prim is introduced in the layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Specifier {
    /// `def` – defines a concrete prim.
    Def,
    /// `over` – overrides an existing prim.
    Over,
    /// `class` – defines an abstract class.
    Class,
}

impl Specifier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Def => "def",
            Self::Over => "over",
            Self::Class => "class",
        }
    }
}

/// A single prim in the USD scene hierarchy.
#[derive(Debug, Clone)]
pub struct Prim {
    /// Specifier keyword.
    pub specifier: Specifier,
    /// USD type string, e.g. `"Xform"`, `"Mesh"`, `"Material"`.
    /// Empty for untyped prims.
    pub type_name: String,
    /// Bare name component (no path prefix).
    pub name: String,
    /// Named attributes.
    pub attributes: Vec<Attribute>,
    /// Named relationships.
    pub relationships: Vec<Relationship>,
    /// Child prims (nested inside `{}`).
    pub children: Vec<Prim>,
}

impl Prim {
    pub fn new(
        specifier: Specifier,
        type_name: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            specifier,
            type_name: type_name.into(),
            name: name.into(),
            attributes: Vec::new(),
            relationships: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Convenience: find an attribute by name.
    pub fn attr(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }

    /// Convenience: find a relationship by name.
    pub fn rel(&self, name: &str) -> Option<&Relationship> {
        self.relationships.iter().find(|r| r.name == name)
    }

    /// Returns the absolute SdfPath of this prim given its parent path.
    pub fn path(&self, parent: &str) -> String {
        if parent == "/" {
            format!("/{}", self.name)
        } else {
            format!("{}/{}", parent, self.name)
        }
    }
}

// ── Stage metadata ────────────────────────────────────────────────────────────

/// Stage-level metadata stored in the `#usda 1.0` header block.
#[derive(Debug, Clone, Default)]
pub struct StageMeta {
    /// `upAxis` — `"Y"` or `"Z"`.
    pub up_axis: Option<String>,
    /// `metersPerUnit` — scene-unit scale.
    pub meters_per_unit: Option<f64>,
    /// `defaultPrim` — path of the default prim.
    pub default_prim: Option<String>,
    /// `doc` — human-readable documentation string.
    pub doc: Option<String>,
    /// Any other metadata key-value pairs not explicitly decoded.
    pub extra: HashMap<String, String>,
}

// ── Document ─────────────────────────────────────────────────────────────────

/// The root of a parsed USDA document.
#[derive(Debug, Clone, Default)]
pub struct UsdDoc {
    /// Stage-level metadata from the header `( … )` block.
    pub meta: StageMeta,
    /// Top-level prims in document order.
    pub root_prims: Vec<Prim>,
}

impl UsdDoc {
    pub fn new() -> Self {
        Self::default()
    }

    /// Find a top-level prim by name.
    pub fn root_prim(&self, name: &str) -> Option<&Prim> {
        self.root_prims.iter().find(|p| p.name == name)
    }

    /// Recursively find a prim by absolute SdfPath like `"/World/Mesh0"`.
    pub fn prim_at_path(&self, path: &str) -> Option<&Prim> {
        let path = path.trim_start_matches('/');
        let mut parts = path.splitn(2, '/');
        let first = parts.next()?;
        let rest = parts.next();
        let top = self.root_prims.iter().find(|p| p.name == first)?;
        match rest {
            None => Some(top),
            Some(tail) => prim_at_tail(top, tail),
        }
    }
}

fn prim_at_tail<'a>(parent: &'a Prim, tail: &str) -> Option<&'a Prim> {
    let mut parts = tail.splitn(2, '/');
    let first = parts.next()?;
    let rest = parts.next();
    let child = parent.children.iter().find(|c| c.name == first)?;
    match rest {
        None => Some(child),
        Some(next) => prim_at_tail(child, next),
    }
}
