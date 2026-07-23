//! Runtime-introspectable import options — the **configurator** system.
//!
//! Enabled by the `configurator` feature. This is a small, engine-agnostic
//! schema/value model so a host application can present a loader's import
//! options at runtime (e.g. render a configurator UI) without compile-time
//! knowledge of each format's options.
//!
//! - A loader describes its options via [`Loader::options_schema`](crate::traits::Loader::options_schema),
//!   returning an [`OptionsSchema`] (an ordered list of [`OptionField`]s).
//! - The host collects chosen values into [`OptionValues`] and calls
//!   [`Loader::load_configured`](crate::traits::Loader::load_configured).
//!
//! Per the same contract as [`LoadOptions`](crate::traits::LoadOptions),
//! loaders honour the options they support and silently ignore the rest —
//! so a schema may advertise fields a given loader (or the host) interprets.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::traits::LoadOptions;

/// Canonical keys for the common [`LoadOptions`] fields, shared across formats.
pub mod keys {
    pub const GENERATE_NORMALS: &str = "generate_normals";
    pub const TRIANGULATE: &str = "triangulate";
    pub const MERGE_VERTICES: &str = "merge_vertices";
    pub const FLIP_UV_V: &str = "flip_uv_v";
    pub const MAX_TEXTURE_SIZE: &str = "max_texture_size";
}

/// A single import-option value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum OptionValue {
    /// A boolean toggle.
    Bool(bool),
    /// An integer value.
    Int(i64),
    /// A floating-point value.
    Float(f64),
    /// Free-form text.
    Text(String),
    /// One of an [`OptionKind::Enum`]'s choices.
    Choice(String),
}

impl OptionValue {
    /// Returns the boolean value, if this is a [`OptionValue::Bool`].
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = self { Some(*b) } else { None }
    }
    /// Returns the value as `i64` (accepts `Int`, or `Float` truncated).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            Self::Float(f) => Some(*f as i64),
            _ => None,
        }
    }
    /// Returns the value as `f64` (accepts `Float`, or `Int` widened).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
    /// Returns the string value, if this is [`OptionValue::Text`] or [`OptionValue::Choice`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) | Self::Choice(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// The kind of an [`OptionField`], plus any UI constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OptionKind {
    /// A boolean toggle (checkbox / switch).
    Bool,
    /// An integer, with optional `min`/`max`/`step` UI constraints.
    Int {
        /// Inclusive minimum, if any.
        min: Option<i64>,
        /// Inclusive maximum, if any.
        max: Option<i64>,
        /// Increment step, if any.
        step: Option<i64>,
    },
    /// A float, with optional `min`/`max`/`step` UI constraints.
    Float {
        /// Inclusive minimum, if any.
        min: Option<f64>,
        /// Inclusive maximum, if any.
        max: Option<f64>,
        /// Increment step, if any.
        step: Option<f64>,
    },
    /// A choice from a fixed set of string values.
    Enum {
        /// The allowed choices.
        choices: Vec<String>,
    },
    /// Free-form text input.
    Text,
}

/// A single configurable import option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionField {
    /// Stable programmatic key (used in [`OptionValues`]).
    pub key: String,
    /// Human-readable label for UI.
    pub label: String,
    /// Longer description / tooltip.
    pub doc: String,
    /// Value kind and UI constraints.
    pub kind: OptionKind,
    /// Default value. Also defines the field's value type.
    pub default: OptionValue,
}

impl OptionField {
    /// Build a boolean field.
    pub fn bool(key: &str, label: &str, doc: &str, default: bool) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            doc: doc.into(),
            kind: OptionKind::Bool,
            default: OptionValue::Bool(default),
        }
    }

    /// Build an integer field with optional `min`/`max`/`step`.
    pub fn int(
        key: &str,
        label: &str,
        doc: &str,
        default: i64,
        min: Option<i64>,
        max: Option<i64>,
        step: Option<i64>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            doc: doc.into(),
            kind: OptionKind::Int { min, max, step },
            default: OptionValue::Int(default),
        }
    }

    /// Build a floating-point field with optional `min`/`max`/`step`.
    pub fn float(
        key: &str,
        label: &str,
        doc: &str,
        default: f64,
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            doc: doc.into(),
            kind: OptionKind::Float { min, max, step },
            default: OptionValue::Float(default),
        }
    }

    /// Build an enum (fixed-choice) field. `default` should be one of `choices`.
    pub fn choice(key: &str, label: &str, doc: &str, default: &str, choices: &[&str]) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            doc: doc.into(),
            kind: OptionKind::Enum {
                choices: choices.iter().map(|s| s.to_string()).collect(),
            },
            default: OptionValue::Choice(default.into()),
        }
    }
}

/// An ordered set of [`OptionField`]s describing a loader's import options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OptionsSchema {
    pub fields: Vec<OptionField>,
}

impl OptionsSchema {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Append a field (builder style).
    pub fn with(mut self, field: OptionField) -> Self {
        self.fields.push(field);
        self
    }

    /// Append several fields (builder style).
    pub fn extend_fields(mut self, fields: impl IntoIterator<Item = OptionField>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// The common options shared by all loaders — mirrors [`LoadOptions`].
    ///
    /// Format crates typically start here and append their own fields.
    pub fn base_load_options() -> Self {
        Self::new()
            .with(OptionField::bool(
                keys::GENERATE_NORMALS,
                "Generate normals",
                "Generate smooth normals for meshes that have none in the file.",
                false,
            ))
            .with(OptionField::bool(
                keys::TRIANGULATE,
                "Triangulate",
                "Triangulate non-triangle polygons (quads, n-gons).",
                false,
            ))
            .with(OptionField::bool(
                keys::MERGE_VERTICES,
                "Merge vertices",
                "Weld duplicate vertices (same position + attributes) into one.",
                false,
            ))
            .with(OptionField::bool(
                keys::FLIP_UV_V,
                "Flip UV (V)",
                "Flip the vertical texture coordinate: v' = 1 − v.",
                false,
            ))
            .with(OptionField::int(
                keys::MAX_TEXTURE_SIZE,
                "Max texture size",
                "Downscale textures to at most this size on their longest axis (0 = no limit).",
                0,
                Some(0),
                Some(16384),
                Some(256),
            ))
    }

    /// Default values for every field, as an [`OptionValues`].
    pub fn default_values(&self) -> OptionValues {
        OptionValues(
            self.fields
                .iter()
                .map(|f| (f.key.clone(), f.default.clone()))
                .collect(),
        )
    }
}

/// A set of chosen import-option values, keyed by [`OptionField::key`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OptionValues(
    /// The chosen values, keyed by field key.
    pub BTreeMap<String, OptionValue>,
);

impl OptionValues {
    /// An empty value set.
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Look up a value by key.
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.0.get(key)
    }

    /// Insert or replace a value.
    pub fn set(&mut self, key: &str, value: OptionValue) {
        self.0.insert(key.into(), value);
    }

    /// Read a boolean value, or `default` if missing / wrong type.
    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        self.get(key).and_then(OptionValue::as_bool).unwrap_or(default)
    }
    /// Read an integer value, or `default` if missing / wrong type.
    pub fn i64_or(&self, key: &str, default: i64) -> i64 {
        self.get(key).and_then(OptionValue::as_i64).unwrap_or(default)
    }
    /// Read a float value, or `default` if missing / wrong type.
    pub fn f64_or(&self, key: &str, default: f64) -> f64 {
        self.get(key).and_then(OptionValue::as_f64).unwrap_or(default)
    }
    /// Read a string value, or `default` if missing / wrong type.
    pub fn str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).and_then(OptionValue::as_str).unwrap_or(default)
    }

    /// Map the common keys onto a [`LoadOptions`]. Format-specific keys are left
    /// for the loader (or host) to interpret. `base_dir` is not exposed as a
    /// configurator field and is left at its default here.
    pub fn to_load_options(&self) -> LoadOptions {
        let mut o = LoadOptions::default();
        o.generate_normals = self.bool_or(keys::GENERATE_NORMALS, o.generate_normals);
        o.triangulate = self.bool_or(keys::TRIANGULATE, o.triangulate);
        o.merge_vertices = self.bool_or(keys::MERGE_VERTICES, o.merge_vertices);
        o.flip_uv_v = self.bool_or(keys::FLIP_UV_V, o.flip_uv_v);
        let mts = self.i64_or(keys::MAX_TEXTURE_SIZE, 0);
        o.max_texture_size = if mts > 0 { Some(mts as u32) } else { None };
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_schema_has_the_common_fields() {
        let s = OptionsSchema::base_load_options();
        let ks: Vec<&str> = s.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(s.fields.len(), 5);
        assert!(ks.contains(&keys::GENERATE_NORMALS));
        assert!(ks.contains(&keys::MAX_TEXTURE_SIZE));
    }

    #[test]
    fn default_values_reflect_field_defaults() {
        let v = OptionsSchema::base_load_options().default_values();
        assert!(!v.bool_or(keys::TRIANGULATE, true));
        assert_eq!(v.i64_or(keys::MAX_TEXTURE_SIZE, -1), 0);
    }

    #[test]
    fn to_load_options_maps_common_keys() {
        let mut v = OptionValues::new();
        v.set(keys::GENERATE_NORMALS, OptionValue::Bool(true));
        v.set(keys::MAX_TEXTURE_SIZE, OptionValue::Int(2048));
        let o = v.to_load_options();
        assert!(o.generate_normals);
        assert_eq!(o.max_texture_size, Some(2048));

        // 0 means "no limit" -> None
        let mut v0 = OptionValues::new();
        v0.set(keys::MAX_TEXTURE_SIZE, OptionValue::Int(0));
        assert_eq!(v0.to_load_options().max_texture_size, None);
    }

    #[test]
    fn values_survive_a_serde_json_roundtrip() {
        let mut v = OptionValues::new();
        v.set("up_axis", OptionValue::Choice("Z".into()));
        v.set(keys::FLIP_UV_V, OptionValue::Bool(true));
        v.set("import_scale", OptionValue::Float(0.01));
        let json = serde_json::to_string(&v).unwrap();
        let back: OptionValues = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn schema_survives_a_serde_json_roundtrip() {
        let s = OptionsSchema::base_load_options();
        let json = serde_json::to_string(&s).unwrap();
        let back: OptionsSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
