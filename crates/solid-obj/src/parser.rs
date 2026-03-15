//! Wavefront OBJ / MTL parser internals.
//!
//! Produces an [`ObjData`] document that [`crate::convert`] then maps to a
//! `solid_rs::Scene`.

use std::collections::HashMap;

// ── Document ──────────────────────────────────────────────────────────────────

/// Top-level result of parsing a `.obj` file.
#[derive(Debug, Default)]
pub(crate) struct ObjData {
    /// Source positions   (`v`)
    pub positions: Vec<[f32; 3]>,
    /// Source normals      (`vn`)
    pub normals:   Vec<[f32; 3]>,
    /// Source UV coords    (`vt`)
    pub uvs:       Vec<[f32; 2]>,
    /// Groups / objects, each becoming one mesh in the scene
    pub groups:    Vec<ObjGroup>,
    /// `mtllib` lines (relative file names)
    pub mtllibs:   Vec<String>,
}

/// A named object / group inside an OBJ file.
#[derive(Debug)]
pub(crate) struct ObjGroup {
    pub name:      String,
    /// Ordered list of face runs; each run may switch material mid-group
    pub face_runs: Vec<FaceRun>,
}

/// A contiguous block of faces that all use the same material.
#[derive(Debug)]
pub(crate) struct FaceRun {
    /// Material name from `usemtl` (empty string = no material)
    pub material: String,
    pub faces:    Vec<ObjFace>,
}

/// A single polygon (3-or-more vertex references).
#[derive(Debug)]
pub(crate) struct ObjFace {
    /// Each element is `(pos_index, uv_index, normal_index)`.
    /// Indices are already 0-based and are `None` where the attribute was
    /// omitted in the face definition.
    pub refs: Vec<(usize, Option<usize>, Option<usize>)>,
}

// ── MTL document ─────────────────────────────────────────────────────────────

/// Parsed contents of a `.mtl` material library.
#[derive(Debug, Default)]
pub(crate) struct MtlData {
    pub materials: HashMap<String, MtlMaterial>,
}

#[derive(Debug, Clone)]
pub(crate) struct MtlMaterial {
    pub name:        String,
    /// Ambient colour (`Ka`)
    pub ka:          [f32; 3],
    /// Diffuse colour (`Kd`) — mapped to `base_color_factor`
    pub kd:          [f32; 3],
    /// Specular colour (`Ks`)
    pub ks:          [f32; 3],
    /// Emissive colour (`Ke`)
    pub ke:          [f32; 3],
    /// Dissolve / opacity (`d`) — 1.0 = fully opaque
    pub dissolve:    f32,
    /// Specular exponent (`Ns`) — 0 … 1000
    pub ns:          f32,
    /// Diffuse texture (`map_Kd`)
    pub map_kd:      Option<String>,
    /// Specular texture (`map_Ks`)
    pub map_ks:      Option<String>,
    /// Normal / bump map (`map_bump`, `bump`, `norm`)
    pub map_bump:    Option<String>,
    /// Roughness map (`map_Ns` or `map_Pr`)
    pub map_roughness: Option<String>,
}

impl Default for MtlMaterial {
    fn default() -> Self {
        Self {
            name:          String::new(),
            ka:            [0.2; 3],
            kd:            [0.8; 3],
            ks:            [0.0; 3],
            ke:            [0.0; 3],
            dissolve:      1.0,
            ns:            32.0,
            map_kd:        None,
            map_ks:        None,
            map_bump:      None,
            map_roughness: None,
        }
    }
}

// ── OBJ Parser ────────────────────────────────────────────────────────────────

/// Parse a Wavefront OBJ text string into [`ObjData`].
pub(crate) fn parse_obj(src: &str) -> ObjData {
    let mut data = ObjData::default();
    let mut current_group = ObjGroup { name: String::from("default"), face_runs: Vec::new() };
    let mut current_material = String::new();
    let mut current_faces: Vec<ObjFace> = Vec::new();

    for raw_line in src.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        let mut tokens = line.splitn(2, |c: char| c.is_whitespace());
        let keyword = tokens.next().unwrap_or("").to_ascii_lowercase();
        let rest    = tokens.next().unwrap_or("").trim();

        match keyword.as_str() {
            "v" => {
                if let Some(p) = parse_vec3(rest) { data.positions.push(p); }
            }
            "vn" => {
                if let Some(n) = parse_vec3(rest) { data.normals.push(n); }
            }
            "vt" => {
                if let Some(uv) = parse_vec2(rest) { data.uvs.push(uv); }
            }
            "f" => {
                if let Some(face) = parse_face(rest, &data) {
                    current_faces.push(face);
                }
            }
            "usemtl" => {
                if !current_faces.is_empty() {
                    current_group.face_runs.push(FaceRun {
                        material: current_material.clone(),
                        faces:    std::mem::take(&mut current_faces),
                    });
                }
                current_material = rest.to_owned();
            }
            "g" | "o" => {
                // Flush current faces before switching group
                if !current_faces.is_empty() {
                    current_group.face_runs.push(FaceRun {
                        material: current_material.clone(),
                        faces:    std::mem::take(&mut current_faces),
                    });
                }
                if !current_group.face_runs.is_empty() {
                    data.groups.push(current_group);
                }
                let name = if rest.is_empty() { "group".to_owned() } else { rest.to_owned() };
                current_group = ObjGroup { name, face_runs: Vec::new() };
            }
            "mtllib" => {
                for lib in rest.split_whitespace() {
                    data.mtllibs.push(lib.to_owned());
                }
            }
            _ => {}
        }
    }

    // Flush final group
    if !current_faces.is_empty() {
        current_group.face_runs.push(FaceRun {
            material: current_material,
            faces:    current_faces,
        });
    }
    if !current_group.face_runs.is_empty() || data.groups.is_empty() {
        data.groups.push(current_group);
    }

    data
}

// ── MTL Parser ────────────────────────────────────────────────────────────────

/// Parse a Wavefront MTL text string into [`MtlData`].
pub(crate) fn parse_mtl(src: &str) -> MtlData {
    let mut out  = MtlData::default();
    let mut cur: Option<MtlMaterial> = None;

    for raw_line in src.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        let mut tokens = line.splitn(2, |c: char| c.is_whitespace());
        let keyword = tokens.next().unwrap_or("").to_ascii_lowercase();
        let rest    = tokens.next().unwrap_or("").trim();

        match keyword.as_str() {
            "newmtl" => {
                if let Some(m) = cur.take() { out.materials.insert(m.name.clone(), m); }
                let mut m = MtlMaterial::default();
                m.name = rest.to_owned();
                cur = Some(m);
            }
            _ if cur.is_none() => {}
            "ka" => {
                if let Some(v) = parse_vec3(rest) { cur.as_mut().unwrap().ka = v; }
            }
            "kd" => {
                if let Some(v) = parse_vec3(rest) { cur.as_mut().unwrap().kd = v; }
            }
            "ks" => {
                if let Some(v) = parse_vec3(rest) { cur.as_mut().unwrap().ks = v; }
            }
            "ke" => {
                if let Some(v) = parse_vec3(rest) { cur.as_mut().unwrap().ke = v; }
            }
            "d" => {
                if let Ok(v) = rest.parse::<f32>() { cur.as_mut().unwrap().dissolve = v; }
            }
            "tr" => {
                // Tr = 1 - d
                if let Ok(v) = rest.parse::<f32>() { cur.as_mut().unwrap().dissolve = 1.0 - v; }
            }
            "ns" => {
                if let Ok(v) = rest.parse::<f32>() { cur.as_mut().unwrap().ns = v; }
            }
            "map_kd" => {
                cur.as_mut().unwrap().map_kd = Some(tex_path(rest));
            }
            "map_ks" => {
                cur.as_mut().unwrap().map_ks = Some(tex_path(rest));
            }
            "map_bump" | "bump" | "norm" => {
                // Skip `-bm` and other option flags
                let path = strip_options(rest);
                if !path.is_empty() {
                    cur.as_mut().unwrap().map_bump = Some(path.to_owned());
                }
            }
            "map_ns" | "map_pr" => {
                cur.as_mut().unwrap().map_roughness = Some(tex_path(rest));
            }
            _ => {}
        }
    }
    if let Some(m) = cur { out.materials.insert(m.name.clone(), m); }
    out
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_vec3(s: &str) -> Option<[f32; 3]> {
    let mut it = s.split_whitespace();
    let x = it.next()?.parse().ok()?;
    let y = it.next()?.parse().ok()?;
    let z = it.next()?.parse().ok()?;
    Some([x, y, z])
}

fn parse_vec2(s: &str) -> Option<[f32; 2]> {
    let mut it = s.split_whitespace();
    let x = it.next()?.parse().ok()?;
    let y = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    Some([x, y])
}

/// Resolve a 1-based OBJ index (may be negative) to a 0-based index.
fn resolve_idx(raw: i64, len: usize) -> Option<usize> {
    if raw == 0 { return None; }
    let idx = if raw < 0 {
        (len as i64 + raw) as usize
    } else {
        (raw - 1) as usize
    };
    if idx < len { Some(idx) } else { None }
}

/// Parse an OBJ face line like `1/2/3 4//5 6`.
fn parse_face(rest: &str, data: &ObjData) -> Option<ObjFace> {
    let mut refs = Vec::new();
    for token in rest.split_whitespace() {
        let mut parts = token.splitn(3, '/');
        let vi_raw: i64  = parts.next()?.parse().ok()?;
        let vt_raw: Option<i64> = parts.next().and_then(|s| if s.is_empty() { None } else { s.parse().ok() });
        let vn_raw: Option<i64> = parts.next().and_then(|s| if s.is_empty() { None } else { s.parse().ok() });

        let vi = resolve_idx(vi_raw, data.positions.len())?;
        let vt = vt_raw.and_then(|i| resolve_idx(i, data.uvs.len()));
        let vn = vn_raw.and_then(|i| resolve_idx(i, data.normals.len()));

        refs.push((vi, vt, vn));
    }
    if refs.len() >= 3 { Some(ObjFace { refs }) } else { None }
}

/// Strip leading option flags (`-bm 1.0`, `-s 1 1`, etc.) from a texture path.
fn strip_options(s: &str) -> &str {
    let mut tokens = s.split_whitespace().peekable();
    loop {
        match tokens.peek() {
            Some(&t) if t.starts_with('-') => {
                tokens.next(); // consume the flag
                tokens.next(); // consume the value
            }
            _ => break,
        }
    }
    // Return the remainder of the original string starting at the last token position
    // Simple approach: just find the last whitespace-delimited token that doesn't start with '-'
    for token in s.split_whitespace().rev() {
        if !token.starts_with('-') {
            // Check it's not a numeric flag value
            if token.parse::<f64>().is_err() {
                return token;
            }
        }
    }
    s
}

fn tex_path(s: &str) -> String {
    // The path is the last non-option token on the line
    s.split_whitespace()
        .filter(|t| !t.starts_with('-'))
        .last()
        .unwrap_or(s)
        .to_owned()
}
