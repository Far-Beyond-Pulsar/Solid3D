//! USDA file saver.

use std::io::Write;

use solid_rs::{
    scene::scene::Scene,
    traits::{FormatInfo, SaveOptions, Saver},
    SolidError,
};

use crate::document::{Attribute, Prim, Relationship, UsdDoc, UsdValue};
use crate::{convert, USD_FORMAT};

pub struct UsdSaver;

impl Saver for UsdSaver {
    fn format_info(&self) -> &'static FormatInfo {
        &USD_FORMAT
    }

    fn save(
        &self,
        scene: &Scene,
        writer: &mut dyn Write,
        _options: &SaveOptions,
    ) -> Result<(), SolidError> {
        let doc = convert::scene_to_doc(scene)?;
        write_doc(&doc, writer)
    }
}

// ── USDA serialiser ───────────────────────────────────────────────────────────

fn write_doc(doc: &UsdDoc, w: &mut dyn Write) -> Result<(), SolidError> {
    let mut out = String::new();

    out.push_str("#usda 1.0\n");

    // Stage metadata
    out.push('(');
    out.push('\n');
    if let Some(axis) = &doc.meta.up_axis {
        out.push_str(&format!("    upAxis = \"{axis}\"\n"));
    }
    if let Some(mpu) = doc.meta.meters_per_unit {
        out.push_str(&format!("    metersPerUnit = {mpu}\n"));
    }
    if let Some(dp) = &doc.meta.default_prim {
        out.push_str(&format!("    defaultPrim = \"{dp}\"\n"));
    }
    if let Some(doc_str) = &doc.meta.doc {
        out.push_str(&format!("    doc = \"\"\"{doc_str}\"\"\"\n"));
    }
    out.push_str(")\n\n");

    for prim in &doc.root_prims {
        write_prim(prim, 0, &mut out);
        out.push('\n');
    }

    w.write_all(out.as_bytes()).map_err(SolidError::Io)
}

fn write_prim(prim: &Prim, indent: usize, out: &mut String) {
    let pad = "    ".repeat(indent);

    // `def TypeName "Name"` or `def "Name"` for untyped prims
    if prim.type_name.is_empty() {
        out.push_str(&format!(
            "{pad}{} \"{}\" {{\n",
            prim.specifier.as_str(),
            prim.name
        ));
    } else {
        out.push_str(&format!(
            "{pad}{} {} \"{}\" {{\n",
            prim.specifier.as_str(),
            prim.type_name,
            prim.name,
        ));
    }

    let inner_pad = "    ".repeat(indent + 1);

    // Attributes
    for attr in &prim.attributes {
        write_attr(attr, &inner_pad, out);
    }

    // Relationships
    for rel in &prim.relationships {
        write_rel(rel, &inner_pad, out);
    }

    // Children
    for child in &prim.children {
        out.push('\n');
        write_prim(child, indent + 1, out);
    }

    out.push_str(&format!("{pad}}}\n"));
}

fn write_attr(attr: &Attribute, pad: &str, out: &mut String) {
    let uniform_kw = if attr.uniform { "uniform " } else { "" };
    let val_str = match &attr.value {
        None => return, // declaration only — skip for now
        Some(v) => format_value(v),
    };
    out.push_str(&format!(
        "{pad}{uniform_kw}{} {} = {val_str}\n",
        attr.type_name, attr.name,
    ));
}

fn write_rel(rel: &Relationship, pad: &str, out: &mut String) {
    match &rel.target {
        Some(t) => out.push_str(&format!("{pad}rel {} = <{t}>\n", rel.name)),
        None => out.push_str(&format!("{pad}rel {}\n", rel.name)),
    }
}

fn format_value(v: &UsdValue) -> String {
    match v {
        UsdValue::Bool(b) => b.to_string(),
        UsdValue::Int(i) => i.to_string(),
        UsdValue::Float(f) => format_float(*f),
        UsdValue::String(s) => format!("\"{s}\""),
        UsdValue::Token(s) => format!("\"{s}\""),
        UsdValue::Asset(s) => format!("@{s}@"),
        UsdValue::Vec2f([a, b]) => format!("({}, {})", format_float(*a), format_float(*b)),
        UsdValue::Vec3f([a, b, c]) => format!(
            "({}, {}, {})",
            format_float(*a),
            format_float(*b),
            format_float(*c)
        ),
        UsdValue::Vec4f([a, b, c, d]) => format!(
            "({}, {}, {}, {})",
            format_float(*a),
            format_float(*b),
            format_float(*c),
            format_float(*d)
        ),
        UsdValue::Matrix4d(m) => {
            let rows: Vec<String> = m
                .iter()
                .map(|row| {
                    let cols: Vec<String> = row.iter().map(|x| format_float(*x)).collect();
                    format!("({})", cols.join(", "))
                })
                .collect();
            format!("({})", rows.join(", "))
        }
        UsdValue::IntArray(arr) => format!(
            "[{}]",
            arr.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        UsdValue::FloatArray(arr) => format!(
            "[{}]",
            arr.iter()
                .map(|f| format_float(*f))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        UsdValue::Vec3fArray(arr) => {
            if arr.is_empty() {
                return "[]".into();
            }
            let items: Vec<String> = arr
                .iter()
                .map(|[a, b, c]| {
                    format!(
                        "({}, {}, {})",
                        format_float(*a),
                        format_float(*b),
                        format_float(*c)
                    )
                })
                .collect();
            format!("[{}]", items.join(", "))
        }
        UsdValue::Vec2fArray(arr) => {
            if arr.is_empty() {
                return "[]".into();
            }
            let items: Vec<String> = arr
                .iter()
                .map(|[a, b]| format!("({}, {})", format_float(*a), format_float(*b)))
                .collect();
            format!("[{}]", items.join(", "))
        }
        UsdValue::TokenArray(arr) => {
            let items: Vec<String> = arr.iter().map(|s| format!("\"{s}\"")).collect();
            format!("[{}]", items.join(", "))
        }
        UsdValue::StringArray(arr) => {
            let items: Vec<String> = arr.iter().map(|s| format!("\"{s}\"")).collect();
            format!("[{}]", items.join(", "))
        }
    }
}

/// Format a float cleanly: use integer representation when the value is whole,
/// otherwise use up to 6 significant digits.
fn format_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{f:.1}")
    } else {
        // trim trailing zeros
        let s = format!("{f:.6}");
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    }
}
