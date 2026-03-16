//! USDC (USD binary Crate) decoder.
//!
//! Implements enough of the OpenUSD Crate format to reconstruct geometry,
//! materials, and scene hierarchy from files exported by DCC tools.
//!
//! ## Format overview
//!
//! ```text
//! Offset 0  │ Bootstrap (88 bytes): magic + version + tocOffset
//! ...        │ Sections at their respective file offsets
//! tocOffset  │ TOC: uint64 count + count × Section{name[16], start, size}
//! ```
//!
//! Sections: TOKENS · STRINGS · FIELDS · FIELDSETS · PATHS · SPECS

use std::collections::HashMap;
use solid_rs::SolidError;

use crate::document::{Attribute, Prim, Relationship, Specifier, StageMeta, UsdDoc, UsdValue};

// ── Magic / bootstrap ─────────────────────────────────────────────────────────

const MAGIC: &[u8; 8] = b"PXR-USDC";
const BOOTSTRAP_SIZE: usize = 88;
// toc offset is at byte 16 of the bootstrap (after magic[8] + version[8])
const TOC_OFFSET_POS: usize = 16;

// ── ValueRep bit layout ───────────────────────────────────────────────────────

const VR_IS_ARRAY:      u64 = 1 << 63;
const VR_IS_INLINED:    u64 = 1 << 62;
const VR_IS_COMPRESSED: u64 = 1 << 61;
const VR_TYPE_SHIFT:    u32 = 48;
const VR_TYPE_MASK:     u64 = 0xFF;
const VR_PAYLOAD_MASK:  u64 = (1u64 << 48) - 1;

// ── USD value type codes ──────────────────────────────────────────────────────

const TY_BOOL:       u8 = 1;
const TY_INT:        u8 = 3;
const TY_UINT:       u8 = 4;
const TY_HALF:       u8 = 7;
const TY_FLOAT:      u8 = 8;
const TY_DOUBLE:     u8 = 9;
const TY_STRING:     u8 = 11;
const TY_TOKEN:      u8 = 12;
const TY_ASSET:      u8 = 13;
const TY_MATRIX4D:   u8 = 16;
const TY_QUATF:      u8 = 18;
const TY_VEC2F:      u8 = 21;
const TY_VEC3F:      u8 = 25;
const TY_VEC4F:      u8 = 29;
const TY_VEC3I:      u8 = 27;
const TY_VEC2D:      u8 = 20;
const TY_VEC3D:      u8 = 24;
const TY_SPECIFIER:  u8 = 39;
const TY_VARIABILITY:u8 = 41;
const TY_TIMESAMPLES:u8 = 43;
const TY_DICT:       u8 = 32;

// ── Spec types ────────────────────────────────────────────────────────────────

const SPEC_ATTRIBUTE:  u32 = 1;
const SPEC_PRIM:       u32 = 6;
const SPEC_PSEUDO_ROOT:u32 = 7;
const SPEC_RELATIONSHIP:u32 = 8;

// ── Public entry ─────────────────────────────────────────────────────────────

/// Parse a USDC binary buffer and return a [`UsdDoc`].
pub fn read(data: &[u8]) -> Result<UsdDoc, SolidError> {
    if data.len() < BOOTSTRAP_SIZE {
        return Err(SolidError::parse("USDC: file too short for bootstrap"));
    }
    if &data[..8] != MAGIC {
        return Err(SolidError::parse("USDC: invalid magic"));
    }

    let toc_offset = u64::from_le_bytes(data[TOC_OFFSET_POS..TOC_OFFSET_POS+8].try_into().unwrap()) as usize;
    let sections   = read_toc(data, toc_offset)?;

    let tokens    = read_tokens_section(data, section_slice(data, &sections, "TOKENS")?)?;
    let strings   = read_strings_section(data, section_slice(data, &sections, "STRINGS")?, &tokens)?;
    let fields    = read_fields_section(data, section_slice(data, &sections, "FIELDS")?)?;
    let fieldsets = read_fieldsets_section(section_slice(data, &sections, "FIELDSETS")?)?;
    let paths     = read_paths_section(section_slice(data, &sections, "PATHS")?, &tokens)?;
    let specs     = read_specs_section(section_slice(data, &sections, "SPECS")?)?;

    build_doc(data, &paths, &specs, &fields, &fieldsets, &tokens, &strings)
}

// ── TOC ───────────────────────────────────────────────────────────────────────

fn read_toc(data: &[u8], offset: usize) -> Result<HashMap<String, (usize, usize)>, SolidError> {
    let mut pos = offset;
    let count = ru64(data, &mut pos)? as usize;
    let mut map = HashMap::new();
    for _ in 0..count {
        let name_bytes = rbytes(data, &mut pos, 16)?;
        let null = name_bytes.iter().position(|&b| b == 0).unwrap_or(16);
        let name = String::from_utf8_lossy(&name_bytes[..null]).into_owned();
        let start = ri64(data, &mut pos)? as usize;
        let size  = ri64(data, &mut pos)? as usize;
        map.insert(name, (start, size));
    }
    Ok(map)
}

fn section_slice<'a>(
    data: &'a [u8],
    sections: &HashMap<String, (usize, usize)>,
    name: &str,
) -> Result<&'a [u8], SolidError> {
    let &(start, size) = sections.get(name)
        .ok_or_else(|| SolidError::parse(format!("USDC: missing section {name}")))?;
    data.get(start..start+size)
        .ok_or_else(|| SolidError::parse(format!("USDC: section {name} out of bounds")))
}

// ── TOKENS section ────────────────────────────────────────────────────────────

fn read_tokens_section(file: &[u8], sec: &[u8]) -> Result<Vec<String>, SolidError> {
    let mut pos = 0;
    let num_tokens      = ru64(sec, &mut pos)? as usize;
    let uncompressed_sz = ru64(sec, &mut pos)? as usize;
    let compressed_sz   = ru64(sec, &mut pos)? as usize;
    let compressed      = &sec[pos..pos+compressed_sz];
    let _ = file;

    let raw = lz4_decompress(compressed, uncompressed_sz)?;

    let mut tokens = Vec::with_capacity(num_tokens);
    let mut i = 0;
    while tokens.len() < num_tokens && i < raw.len() {
        let end = raw[i..].iter().position(|&b| b == 0).unwrap_or(raw.len() - i);
        tokens.push(String::from_utf8_lossy(&raw[i..i+end]).into_owned());
        i += end + 1;
    }
    Ok(tokens)
}

// ── STRINGS section ───────────────────────────────────────────────────────────

fn read_strings_section(
    _file: &[u8],
    sec: &[u8],
    tokens: &[String],
) -> Result<Vec<String>, SolidError> {
    let mut pos = 0;
    let num = ru64(sec, &mut pos)? as usize;
    let mut strings = Vec::with_capacity(num);
    for _ in 0..num {
        let tok_idx = ru32(sec, &mut pos)? as usize;
        strings.push(tokens.get(tok_idx).cloned().unwrap_or_default());
    }
    Ok(strings)
}

// ── FIELDS section ────────────────────────────────────────────────────────────

/// Returns `Vec<(name_token_idx, value_rep_u64)>`
fn read_fields_section(file: &[u8], sec: &[u8]) -> Result<Vec<(u32, u64)>, SolidError> {
    let mut pos = 0usize;
    let num = ru64(sec, &mut pos)? as usize;
    if num == 0 { return Ok(vec![]); }

    // name token indices — compressed uint32 array
    let name_toks = read_compressed_u32(sec, &mut pos, num)?;

    // value reps — raw uint64 (NOT compressed)
    let mut value_reps = Vec::with_capacity(num);
    for _ in 0..num {
        value_reps.push(ru64(sec, &mut pos)?);
    }
    let _ = file;

    Ok(name_toks.into_iter().zip(value_reps).collect())
}

// ── FIELDSETS section ─────────────────────────────────────────────────────────

fn read_fieldsets_section(sec: &[u8]) -> Result<Vec<Vec<u32>>, SolidError> {
    let mut pos = 0usize;
    let num_ints = ru64(sec, &mut pos)? as usize;
    if num_ints == 0 { return Ok(vec![]); }

    let flat = read_compressed_u32(sec, &mut pos, num_ints)?;

    let mut sets = Vec::new();
    let mut current = Vec::new();
    for v in flat {
        if v == 0xFFFF_FFFF {
            sets.push(std::mem::take(&mut current));
        } else {
            current.push(v);
        }
    }
    if !current.is_empty() { sets.push(current); }
    Ok(sets)
}

// ── PATHS section ─────────────────────────────────────────────────────────────

fn read_paths_section(sec: &[u8], tokens: &[String]) -> Result<Vec<String>, SolidError> {
    let mut pos = 0usize;
    let num = ru64(sec, &mut pos)? as usize;
    if num == 0 { return Ok(vec![]); }

    let raw_tok_indices = read_compressed_u32(sec, &mut pos, num)?;
    let jump_raw        = read_compressed_u32(sec, &mut pos, num)?;

    let mut paths  = Vec::with_capacity(num);
    let mut stack: Vec<usize> = Vec::new(); // indices into `paths`

    for i in 0..num {
        let raw = raw_tok_indices[i];
        let is_prop = (raw & 1) != 0;
        let tok_idx = (raw >> 1) as usize;

        // jump is stored as u32 but semantically may represent "pop count"
        let jump = jump_raw[i] as usize;

        if tok_idx == 0 && !is_prop {
            // Root "/"
            paths.push("/".to_string());
            stack.clear();
            stack.push(paths.len() - 1);
            continue;
        }

        // Pop `jump` levels
        let new_len = stack.len().saturating_sub(jump);
        stack.truncate(new_len);

        let parent = stack.last().map(|&idx| paths[idx].as_str()).unwrap_or("/");
        let elem   = tokens.get(tok_idx).map(|s| s.as_str()).unwrap_or("_");

        let new_path = if is_prop {
            format!("{parent}.{elem}")
        } else if parent == "/" {
            format!("/{elem}")
        } else {
            format!("{parent}/{elem}")
        };

        let new_idx = paths.len();
        paths.push(new_path);
        stack.push(new_idx);
    }

    Ok(paths)
}

// ── SPECS section ─────────────────────────────────────────────────────────────

/// Returns `Vec<(path_idx, fieldset_idx, spec_type)>`
fn read_specs_section(sec: &[u8]) -> Result<Vec<(u32, u32, u32)>, SolidError> {
    let mut pos = 0usize;
    let num = ru64(sec, &mut pos)? as usize;
    if num == 0 { return Ok(vec![]); }

    let path_idxs     = read_compressed_u32(sec, &mut pos, num)?;
    let fieldset_idxs = read_compressed_u32(sec, &mut pos, num)?;
    let spec_types    = read_compressed_u32(sec, &mut pos, num)?;

    Ok((0..num).map(|i| (path_idxs[i], fieldset_idxs[i], spec_types[i])).collect())
}

// ── Prim tree reconstruction ──────────────────────────────────────────────────

fn build_doc(
    file_data: &[u8],
    paths:     &[String],
    specs:     &[(u32, u32, u32)],
    fields:    &[(u32, u64)],
    fieldsets: &[Vec<u32>],
    tokens:    &[String],
    strings:   &[String],
) -> Result<UsdDoc, SolidError> {
    let mut doc = UsdDoc::new();

    // ── Pass 1: collect attribute/relationship specs by parent prim path ──────
    let mut prim_attrs: HashMap<String, Vec<Attribute>>   = HashMap::new();
    let mut prim_rels:  HashMap<String, Vec<Relationship>> = HashMap::new();

    for &(path_idx, fs_idx, spec_type) in specs {
        let path = match paths.get(path_idx as usize) {
            Some(p) => p.clone(),
            None    => continue,
        };

        match spec_type {
            SPEC_ATTRIBUTE => {
                if let Some(dot) = path.rfind('.') {
                    let parent = path[..dot].to_string();
                    let name   = path[dot+1..].to_string();
                    if let Some(attr) = decode_attribute_spec(
                        &name, fs_idx as usize, fieldsets, fields, tokens, strings, file_data,
                    ) {
                        prim_attrs.entry(parent).or_default().push(attr);
                    }
                }
            }
            SPEC_RELATIONSHIP => {
                if let Some(dot) = path.rfind('.') {
                    let parent = path[..dot].to_string();
                    let name   = path[dot+1..].to_string();
                    let rel    = decode_relationship_spec(&name, fs_idx as usize, fieldsets, fields, tokens, file_data);
                    prim_rels.entry(parent).or_default().push(rel);
                }
            }
            _ => {}
        }
    }

    // ── Pass 2: reconstruct prim tree (DFS order) ─────────────────────────────
    // Stack holds (path_string, prim)
    let mut stack: Vec<(String, Prim)> = Vec::new();

    let depth_of = |p: &str| -> usize {
        if p == "/" { 0 } else { p.matches('/').count() }
    };

    for &(path_idx, fs_idx, spec_type) in specs {
        let path = match paths.get(path_idx as usize) {
            Some(p) => p.clone(),
            None    => continue,
        };

        match spec_type {
            SPEC_PSEUDO_ROOT => {
                // "/" root — initialise the stack; we'll use root prims directly
                stack.push(("/".to_string(), Prim::new(Specifier::Def, "", "")));
            }
            SPEC_PRIM => {
                if path == "/" { continue; }

                let depth = depth_of(&path);

                // Pop prim children deeper than this depth, attaching them
                while stack.len() > depth {
                    let (old_path, old_prim) = stack.pop().unwrap();
                    if let Some((_, parent)) = stack.last_mut() {
                        parent.children.push(old_prim);
                    } else {
                        // Direct root child
                        doc.root_prims.push(old_prim);
                        let _ = old_path;
                    }
                }

                let (specifier, type_name) = decode_prim_fields(
                    fs_idx as usize, fieldsets, fields, tokens,
                );
                let name = path.split('/').last().unwrap_or("_");
                let mut prim = Prim::new(specifier, type_name, name);

                // Attach collected attributes/relationships
                prim.attributes  = prim_attrs.remove(&path).unwrap_or_default();
                prim.relationships = prim_rels.remove(&path).unwrap_or_default();

                // Also extract attributes inline in the prim fieldset
                // (e.g. "typeName", "specifier" already decoded; skip them)
                inline_prim_attrs(&mut prim, fs_idx as usize, fieldsets, fields, tokens, strings, file_data);

                stack.push((path, prim));
            }
            _ => {}
        }
    }

    // Drain remaining stack
    while let Some((_, prim)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(prim);
        } else if prim.type_name != "" || !prim.children.is_empty() {
            doc.root_prims.push(prim);
        }
    }

    // Stage metadata — look for the pseudo-root fieldset
    if let Some(&(_, fs_idx, SPEC_PSEUDO_ROOT)) = specs.first() {
        doc.meta = decode_stage_meta(fs_idx as usize, fieldsets, fields, tokens, strings, file_data);
    }

    Ok(doc)
}

// ── Field/attribute decoding ──────────────────────────────────────────────────

fn decode_prim_fields(
    fs_idx:    usize,
    fieldsets: &[Vec<u32>],
    fields:    &[(u32, u64)],
    tokens:    &[String],
) -> (Specifier, String) {
    let mut specifier  = Specifier::Def;
    let mut type_name  = String::new();

    let set = match fieldsets.get(fs_idx) { Some(s) => s, None => return (specifier, type_name) };
    for &fi in set {
        let (name_tok, vrep) = match fields.get(fi as usize) { Some(f) => *f, None => continue };
        let key = tokens.get(name_tok as usize).map(|s| s.as_str()).unwrap_or("");
        match key {
            "specifier" => {
                let payload = vrep & VR_PAYLOAD_MASK;
                specifier = match payload {
                    0 => Specifier::Def,
                    1 => Specifier::Over,
                    2 => Specifier::Class,
                    _ => Specifier::Def,
                };
            }
            "typeName" => {
                let payload = vrep & VR_PAYLOAD_MASK;
                if (vrep & VR_IS_INLINED) != 0 {
                    let ty_code = ((vrep >> VR_TYPE_SHIFT) & VR_TYPE_MASK) as u8;
                    if ty_code == TY_TOKEN {
                        type_name = tokens.get(payload as usize).cloned().unwrap_or_default();
                    }
                }
            }
            _ => {}
        }
    }
    (specifier, type_name)
}

fn inline_prim_attrs(
    prim:      &mut Prim,
    fs_idx:    usize,
    fieldsets: &[Vec<u32>],
    fields:    &[(u32, u64)],
    tokens:    &[String],
    strings:   &[String],
    file_data: &[u8],
) {
    // Some attrs live directly in the prim fieldset under their name
    // e.g. "metersPerUnit", "upAxis", "documentation" at the pseudo-root
    // For mesh prims, geometry attrs are usually in sub-specs, not here.
    let set = match fieldsets.get(fs_idx) { Some(s) => s, None => return };
    for &fi in set {
        let (name_tok, vrep) = match fields.get(fi as usize) { Some(f) => *f, None => continue };
        let key = tokens.get(name_tok as usize).map(|s| s.as_str()).unwrap_or("");
        // Skip structural fields already handled
        if matches!(key, "specifier" | "typeName" | "properties" | "primChildren") { continue; }

        if let Some(val) = decode_value(vrep, file_data, tokens, strings) {
            // Guess the type_name from the type code
            let ty_code = ((vrep >> VR_TYPE_SHIFT) & VR_TYPE_MASK) as u8;
            let type_name = type_name_str(ty_code, vrep);
            prim.attributes.push(Attribute {
                name: key.to_string(),
                type_name: type_name.into(),
                value: Some(val),
                uniform: false,
            });
        }
    }
}

fn decode_attribute_spec(
    name:      &str,
    fs_idx:    usize,
    fieldsets: &[Vec<u32>],
    fields:    &[(u32, u64)],
    tokens:    &[String],
    strings:   &[String],
    file_data: &[u8],
) -> Option<Attribute> {
    let set = fieldsets.get(fs_idx)?;
    let mut type_name = String::new();
    let mut value     = None;
    let mut uniform   = false;

    for &fi in set {
        let (name_tok, vrep) = fields.get(fi as usize).copied()?;
        let key = tokens.get(name_tok as usize).map(|s| s.as_str()).unwrap_or("");
        match key {
            "typeName" => {
                if (vrep & VR_IS_INLINED) != 0 {
                    let ty_code = ((vrep >> VR_TYPE_SHIFT) & VR_TYPE_MASK) as u8;
                    if ty_code == TY_TOKEN {
                        type_name = tokens.get((vrep & VR_PAYLOAD_MASK) as usize).cloned().unwrap_or_default();
                    }
                }
            }
            "variability" => {
                if (vrep & VR_PAYLOAD_MASK) == 1 { uniform = true; }
            }
            "default" => {
                value = decode_value(vrep, file_data, tokens, strings);
                if type_name.is_empty() {
                    let ty_code = ((vrep >> VR_TYPE_SHIFT) & VR_TYPE_MASK) as u8;
                    type_name = type_name_str(ty_code, vrep).into();
                }
            }
            "timeSamples" => {
                // Decode first time sample as the "default" value
                value = decode_time_samples_first(vrep, file_data, tokens, strings);
            }
            _ => {}
        }
    }

    Some(Attribute { name: name.to_string(), type_name, value, uniform })
}

fn decode_relationship_spec(
    name:      &str,
    fs_idx:    usize,
    fieldsets: &[Vec<u32>],
    fields:    &[(u32, u64)],
    tokens:    &[String],
    file_data: &[u8],
) -> Relationship {
    let mut target = None;
    if let Some(set) = fieldsets.get(fs_idx) {
        for &fi in set {
            if let Some(&(name_tok, vrep)) = fields.get(fi as usize) {
                let key = tokens.get(name_tok as usize).map(|s| s.as_str()).unwrap_or("");
                if key == "targetPaths" || key == "connectionPaths" {
                    // Target is a list of SdfPaths stored as a token/string
                    if (vrep & VR_IS_INLINED) != 0 {
                        let ty_code = ((vrep >> VR_TYPE_SHIFT) & VR_TYPE_MASK) as u8;
                        if ty_code == TY_TOKEN {
                            target = tokens.get((vrep & VR_PAYLOAD_MASK) as usize).cloned();
                        }
                    } else {
                        // Read string from file offset
                        let offset = (vrep & VR_PAYLOAD_MASK) as usize;
                        if offset + 8 <= file_data.len() {
                            let mut p = offset;
                            if let Ok(count) = ru64(file_data, &mut p) {
                                if count > 0 {
                                    if let Ok(idx) = ru32(file_data, &mut p) {
                                        target = tokens.get(idx as usize).cloned();
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
                let _ = file_data;
            }
        }
    }
    Relationship { name: name.to_string(), target }
}

fn decode_stage_meta(
    fs_idx:    usize,
    fieldsets: &[Vec<u32>],
    fields:    &[(u32, u64)],
    tokens:    &[String],
    strings:   &[String],
    file_data: &[u8],
) -> StageMeta {
    let mut meta = StageMeta::default();
    let set = match fieldsets.get(fs_idx) { Some(s) => s, None => return meta };

    for &fi in set {
        let (name_tok, vrep) = match fields.get(fi as usize) { Some(f) => *f, None => continue };
        let key = tokens.get(name_tok as usize).map(|s| s.as_str()).unwrap_or("");

        if let Some(val) = decode_value(vrep, file_data, tokens, strings) {
            match key {
                "upAxis"        => meta.up_axis         = val_as_string(&val),
                "defaultPrim"   => meta.default_prim    = val_as_string(&val),
                "doc"           => meta.doc             = val_as_string(&val),
                "metersPerUnit" => meta.meters_per_unit = val_as_f64(&val),
                _ => {}
            }
        }
    }
    meta
}

// ── Value decoding ────────────────────────────────────────────────────────────

fn decode_value(
    vrep:      u64,
    file_data: &[u8],
    tokens:    &[String],
    strings:   &[String],
) -> Option<UsdValue> {
    let ty_code    = ((vrep >> VR_TYPE_SHIFT) & VR_TYPE_MASK) as u8;
    let is_inlined = (vrep & VR_IS_INLINED)    != 0;
    let is_array   = (vrep & VR_IS_ARRAY)       != 0;
    let is_comp    = (vrep & VR_IS_COMPRESSED)  != 0;
    let payload    = vrep & VR_PAYLOAD_MASK;

    if is_array {
        return decode_array_value(ty_code, payload as usize, is_comp, file_data, tokens, strings);
    }

    if is_inlined {
        return decode_inline_value(ty_code, payload, tokens, strings);
    }

    // Non-inline scalar at file offset
    let offset = payload as usize;
    decode_scalar_at(ty_code, offset, file_data, tokens, strings)
}

fn decode_inline_value(
    ty_code: u8,
    payload: u64,
    tokens:  &[String],
    strings: &[String],
) -> Option<UsdValue> {
    match ty_code {
        TY_BOOL       => Some(UsdValue::Bool(payload & 1 != 0)),
        TY_INT        => Some(UsdValue::Int(payload as i32 as i64)),
        TY_UINT       => Some(UsdValue::Int(payload as i64)),
        TY_HALF       => {
            let h = payload as u16;
            Some(UsdValue::Float(half_to_f64(h)))
        }
        TY_FLOAT      => {
            let bits = payload as u32;
            Some(UsdValue::Float(f32::from_bits(bits) as f64))
        }
        TY_TOKEN      => {
            Some(UsdValue::Token(tokens.get(payload as usize).cloned().unwrap_or_default()))
        }
        TY_STRING     => {
            Some(UsdValue::String(strings.get(payload as usize).cloned().unwrap_or_default()))
        }
        TY_ASSET      => {
            Some(UsdValue::Asset(tokens.get(payload as usize).cloned().unwrap_or_default()))
        }
        TY_SPECIFIER  => Some(UsdValue::Int(payload as i64)),
        TY_VARIABILITY=> Some(UsdValue::Int(payload as i64)),
        _ => None,
    }
}

fn decode_scalar_at(
    ty_code:   u8,
    offset:    usize,
    file_data: &[u8],
    tokens:    &[String],
    strings:   &[String],
) -> Option<UsdValue> {
    let d = file_data;
    match ty_code {
        TY_DOUBLE => {
            if offset + 8 > d.len() { return None; }
            Some(UsdValue::Float(f64::from_le_bytes(d[offset..offset+8].try_into().ok()?)))
        }
        TY_FLOAT => {
            if offset + 4 > d.len() { return None; }
            Some(UsdValue::Float(f32::from_bits(u32::from_le_bytes(d[offset..offset+4].try_into().ok()?)) as f64))
        }
        TY_VEC2F | TY_VEC2D => {
            if offset + 8 > d.len() { return None; }
            let a = f32::from_bits(u32::from_le_bytes(d[offset..offset+4].try_into().ok()?));
            let b = f32::from_bits(u32::from_le_bytes(d[offset+4..offset+8].try_into().ok()?));
            Some(UsdValue::Vec2f([a as f64, b as f64]))
        }
        TY_VEC3F | TY_VEC3D | TY_VEC3I => {
            if offset + 12 > d.len() { return None; }
            let x = f32::from_bits(u32::from_le_bytes(d[offset   ..offset+4 ].try_into().ok()?));
            let y = f32::from_bits(u32::from_le_bytes(d[offset+4 ..offset+8 ].try_into().ok()?));
            let z = f32::from_bits(u32::from_le_bytes(d[offset+8 ..offset+12].try_into().ok()?));
            Some(UsdValue::Vec3f([x as f64, y as f64, z as f64]))
        }
        TY_VEC4F | TY_QUATF => {
            if offset + 16 > d.len() { return None; }
            let x = f32::from_bits(u32::from_le_bytes(d[offset   ..offset+4 ].try_into().ok()?));
            let y = f32::from_bits(u32::from_le_bytes(d[offset+4 ..offset+8 ].try_into().ok()?));
            let z = f32::from_bits(u32::from_le_bytes(d[offset+8 ..offset+12].try_into().ok()?));
            let w = f32::from_bits(u32::from_le_bytes(d[offset+12..offset+16].try_into().ok()?));
            Some(UsdValue::Vec4f([x as f64, y as f64, z as f64, w as f64]))
        }
        TY_MATRIX4D => {
            if offset + 128 > d.len() { return None; }
            let mut m = [[0f64; 4]; 4];
            for row in 0..4 {
                for col in 0..4 {
                    let o = offset + (row*4 + col)*8;
                    m[row][col] = f64::from_le_bytes(d[o..o+8].try_into().ok()?);
                }
            }
            Some(UsdValue::Matrix4d(m))
        }
        TY_STRING => {
            if offset + 4 > d.len() { return None; }
            let idx = u32::from_le_bytes(d[offset..offset+4].try_into().ok()?) as usize;
            Some(UsdValue::String(strings.get(idx).cloned().unwrap_or_default()))
        }
        TY_TOKEN | TY_ASSET => {
            if offset + 4 > d.len() { return None; }
            let idx = u32::from_le_bytes(d[offset..offset+4].try_into().ok()?) as usize;
            let s = tokens.get(idx).cloned().unwrap_or_default();
            if ty_code == TY_ASSET { Some(UsdValue::Asset(s)) } else { Some(UsdValue::Token(s)) }
        }
        _ => None,
    }
}

fn decode_array_value(
    ty_code:   u8,
    offset:    usize,
    is_comp:   bool,
    file_data: &[u8],
    tokens:    &[String],
    _strings:  &[String],
) -> Option<UsdValue> {
    let d = file_data;
    if offset + 8 > d.len() { return None; }

    let count = u64::from_le_bytes(d[offset..offset+8].try_into().ok()?) as usize;
    let data_start = offset + 8;

    match ty_code {
        TY_INT | TY_UINT | TY_VEC3I => {
            let bytes = read_maybe_compressed_bytes(d, data_start, count * 4, is_comp)?;
            let vals: Vec<i64> = bytes.chunks_exact(4)
                .map(|c| i32::from_le_bytes(c.try_into().unwrap()) as i64)
                .collect();
            Some(UsdValue::IntArray(vals))
        }
        TY_FLOAT | TY_HALF => {
            let bytes = read_float_bytes(d, data_start, count * 4, is_comp)?;
            let vals: Vec<f64> = bytes.chunks_exact(4)
                .map(|c| f32::from_bits(u32::from_le_bytes(c.try_into().unwrap())) as f64)
                .collect();
            Some(UsdValue::FloatArray(vals))
        }
        TY_DOUBLE => {
            let bytes = read_maybe_compressed_bytes(d, data_start, count * 8, is_comp)?;
            let vals: Vec<f64> = bytes.chunks_exact(8)
                .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            Some(UsdValue::FloatArray(vals))
        }
        TY_VEC2F | TY_VEC2D => {
            let bytes = read_float_bytes(d, data_start, count * 8, is_comp)?;
            let vals: Vec<[f64; 2]> = bytes.chunks_exact(8).map(|c| {
                let a = f32::from_bits(u32::from_le_bytes(c[0..4].try_into().unwrap()));
                let b = f32::from_bits(u32::from_le_bytes(c[4..8].try_into().unwrap()));
                [a as f64, b as f64]
            }).collect();
            Some(UsdValue::Vec2fArray(vals))
        }
        TY_VEC3F | TY_VEC3D => {
            let bytes = read_float_bytes(d, data_start, count * 12, is_comp)?;
            let vals: Vec<[f64; 3]> = bytes.chunks_exact(12).map(|c| {
                let x = f32::from_bits(u32::from_le_bytes(c[0..4].try_into().unwrap()));
                let y = f32::from_bits(u32::from_le_bytes(c[4..8].try_into().unwrap()));
                let z = f32::from_bits(u32::from_le_bytes(c[8..12].try_into().unwrap()));
                [x as f64, y as f64, z as f64]
            }).collect();
            Some(UsdValue::Vec3fArray(vals))
        }
        TY_TOKEN => {
            let bytes = read_maybe_compressed_bytes(d, data_start, count * 4, is_comp)?;
            let toks: Vec<String> = bytes.chunks_exact(4)
                .map(|c| {
                    let idx = u32::from_le_bytes(c.try_into().unwrap()) as usize;
                    tokens.get(idx).cloned().unwrap_or_default()
                })
                .collect();
            Some(UsdValue::TokenArray(toks))
        }
        _ => None,
    }
}

/// Read array bytes, applying optional lz4 decompression.
/// `expected_raw_bytes` is the uncompressed byte count.
fn read_maybe_compressed_bytes(
    d:                  &[u8],
    data_start:         usize,
    expected_raw_bytes: usize,
    is_comp:            bool,
) -> Option<Vec<u8>> {
    if !is_comp {
        if data_start + expected_raw_bytes > d.len() { return None; }
        return Some(d[data_start..data_start + expected_raw_bytes].to_vec());
    }
    // Compressed: uint64 compressedSize + lz4 data
    if data_start + 8 > d.len() { return None; }
    let comp_sz = u64::from_le_bytes(d[data_start..data_start+8].try_into().ok()?) as usize;
    let src = d.get(data_start+8..data_start+8+comp_sz)?;
    lz4_decompress(src, expected_raw_bytes).ok()
}

/// Like `read_maybe_compressed_bytes` but also applies XOR delta decode for float data.
fn read_float_bytes(
    d:                  &[u8],
    data_start:         usize,
    expected_raw_bytes: usize,
    is_comp:            bool,
) -> Option<Vec<u8>> {
    let mut bytes = read_maybe_compressed_bytes(d, data_start, expected_raw_bytes, is_comp)?;
    if is_comp {
        // Undo XOR delta encoding (applied before lz4 in USD)
        let u32s: &mut [u32] = bytemuck_cast_slice_mut(&mut bytes)?;
        let mut prev = 0u32;
        for v in u32s.iter_mut() {
            *v ^= prev;
            prev = *v;
        }
    }
    Some(bytes)
}

/// Zero-copy cast of `&mut [u8]` to `&mut [u32]` (only if len is divisible by 4).
fn bytemuck_cast_slice_mut(bytes: &mut Vec<u8>) -> Option<&mut [u32]> {
    if bytes.len() % 4 != 0 { return None; }
    // SAFETY: u8 and u32 have no validity invariants; alignment ensured by Vec.
    // We re-allocate to guarantee 4-byte alignment.
    let aligned: Vec<u32> = bytes.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    *bytes = aligned.iter().flat_map(|v| v.to_le_bytes()).collect();
    let ptr   = bytes.as_mut_ptr() as *mut u32;
    let len   = bytes.len() / 4;
    // SAFETY: ptr came from Vec<u8> with len%4==0; we own the data.
    Some(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
}

/// Decode the "first" time sample from a timeSamples value rep.
/// Returns the value of the earliest keyframe.
fn decode_time_samples_first(
    vrep:      u64,
    file_data: &[u8],
    tokens:    &[String],
    strings:   &[String],
) -> Option<UsdValue> {
    // TimeSamples are stored as a dict-like structure at file offset.
    // Layout: uint64 count, then for each sample: (double time, ValueRep value)
    let is_inlined = (vrep & VR_IS_INLINED) != 0;
    if is_inlined { return None; }

    let offset = (vrep & VR_PAYLOAD_MASK) as usize;
    if offset + 8 > file_data.len() { return None; }

    let mut pos = offset;
    let count = ru64(file_data, &mut pos).ok()? as usize;
    if count == 0 { return None; }

    // Skip the double time value (8 bytes)
    pos += 8;

    // Read the ValueRep of the first sample
    if pos + 8 > file_data.len() { return None; }
    let sample_vrep = ru64(file_data, &mut pos).ok()?;
    decode_value(sample_vrep, file_data, tokens, strings)
}

// ── Compressed integer reading ────────────────────────────────────────────────

/// Read a count-prefixed, potentially lz4-compressed array of uint32.
///
/// Wire format (after the `count` u64 already consumed by caller):
/// `compressedSize: u64`, then `compressedSize` bytes of either raw or lz4 data.
/// If `compressedSize == count * 4` the data is raw; otherwise lz4.
fn read_compressed_u32(data: &[u8], pos: &mut usize, count: usize) -> Result<Vec<u32>, SolidError> {
    if count == 0 { return Ok(vec![]); }
    let uncompressed_size = count * 4;
    let compressed_size   = ru64(data, pos)? as usize;

    let raw = if compressed_size == uncompressed_size {
        // Not actually compressed — raw bytes
        let bytes = rbytes(data, pos, compressed_size)?;
        bytes.to_vec()
    } else {
        let bytes = rbytes(data, pos, compressed_size)?;
        lz4_decompress(bytes, uncompressed_size)?
    };

    Ok(raw.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect())
}

// ── LZ4 decompression ─────────────────────────────────────────────────────────

fn lz4_decompress(src: &[u8], expected: usize) -> Result<Vec<u8>, SolidError> {
    lz4_flex::block::decompress(src, expected)
        .map_err(|e| SolidError::parse(format!("USDC lz4 decompress: {e}")))
}

// ── Low-level byte readers ────────────────────────────────────────────────────

fn ru8(d: &[u8], pos: &mut usize) -> Result<u8, SolidError> {
    let v = *d.get(*pos).ok_or_else(|| SolidError::parse("USDC: unexpected EOF (u8)"))?;
    *pos += 1;
    Ok(v)
}

fn ru16(d: &[u8], pos: &mut usize) -> Result<u16, SolidError> {
    let s = d.get(*pos..*pos+2).ok_or_else(|| SolidError::parse("USDC: unexpected EOF (u16)"))?;
    *pos += 2;
    Ok(u16::from_le_bytes(s.try_into().unwrap()))
}

fn ru32(d: &[u8], pos: &mut usize) -> Result<u32, SolidError> {
    let s = d.get(*pos..*pos+4).ok_or_else(|| SolidError::parse("USDC: unexpected EOF (u32)"))?;
    *pos += 4;
    Ok(u32::from_le_bytes(s.try_into().unwrap()))
}

fn ru64(d: &[u8], pos: &mut usize) -> Result<u64, SolidError> {
    let s = d.get(*pos..*pos+8).ok_or_else(|| SolidError::parse("USDC: unexpected EOF (u64)"))?;
    *pos += 8;
    Ok(u64::from_le_bytes(s.try_into().unwrap()))
}

fn ri64(d: &[u8], pos: &mut usize) -> Result<i64, SolidError> {
    let s = d.get(*pos..*pos+8).ok_or_else(|| SolidError::parse("USDC: unexpected EOF (i64)"))?;
    *pos += 8;
    Ok(i64::from_le_bytes(s.try_into().unwrap()))
}

fn rbytes<'a>(d: &'a [u8], pos: &mut usize, n: usize) -> Result<&'a [u8], SolidError> {
    let s = d.get(*pos..*pos+n).ok_or_else(|| SolidError::parse(format!("USDC: unexpected EOF ({n} bytes)")))?;
    *pos += n;
    Ok(s)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn half_to_f64(h: u16) -> f64 {
    // Simple IEEE 754 half → double conversion
    let exp  = ((h >> 10) & 0x1F) as i32;
    let mant = (h & 0x3FF) as f64;
    let sign = if h & 0x8000 != 0 { -1.0f64 } else { 1.0f64 };
    if exp == 0 {
        sign * mant * 5.960464477539063e-8  // subnormal
    } else if exp == 31 {
        if mant == 0.0 { sign * f64::INFINITY } else { f64::NAN }
    } else {
        sign * (1.0 + mant / 1024.0) * (2f64.powi(exp - 15))
    }
}

fn type_name_str(ty_code: u8, vrep: u64) -> &'static str {
    let is_array = (vrep & VR_IS_ARRAY) != 0;
    match (ty_code, is_array) {
        (TY_BOOL,  false) => "bool",
        (TY_INT,   false) => "int",
        (TY_INT,   true)  => "int[]",
        (TY_FLOAT, false) => "float",
        (TY_FLOAT, true)  => "float[]",
        (TY_DOUBLE,false) => "double",
        (TY_STRING,false) => "string",
        (TY_TOKEN, false) => "token",
        (TY_TOKEN, true)  => "token[]",
        (TY_ASSET, false) => "asset",
        (TY_VEC2F, false) => "float2",
        (TY_VEC2F, true)  => "texCoord2f[]",
        (TY_VEC3F, false) => "float3",
        (TY_VEC3F, true)  => "point3f[]",
        (TY_VEC4F, false) => "float4",
        (TY_MATRIX4D,false) => "matrix4d",
        _ => "unknown",
    }
}

fn val_as_string(v: &UsdValue) -> Option<String> {
    match v {
        UsdValue::String(s) | UsdValue::Token(s) => Some(s.clone()),
        _ => None,
    }
}

fn val_as_f64(v: &UsdValue) -> Option<f64> {
    match v {
        UsdValue::Float(f) => Some(*f),
        UsdValue::Int(i)   => Some(*i as f64),
        _ => None,
    }
}

// Suppress unused-variable warnings from the `_` prefixed readers
#[allow(dead_code)] fn _ru8_unused(d: &[u8], p: &mut usize)  -> Result<u8,  SolidError> { ru8(d, p) }
#[allow(dead_code)] fn _ru16_unused(d: &[u8], p: &mut usize) -> Result<u16, SolidError> { ru16(d, p) }
