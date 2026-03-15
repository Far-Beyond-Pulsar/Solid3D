//! PLY loader: parses ASCII and binary PLY files into a `solid_rs::Scene`.

use std::io::Read;

use glam::{Vec2, Vec3, Vec4};
use solid_rs::prelude::*;
use solid_rs::scene::Scene;
use solid_rs::{Result, SolidError};

use crate::header::{parse_header, Element, PlyFormat, PropType, ScalarType};
use crate::PLY_FORMAT;

pub struct PlyLoader;

impl Loader for PlyLoader {
    fn format_info(&self) -> &FormatInfo {
        &PLY_FORMAT
    }

    fn load(&self, reader: &mut dyn ReadSeek, options: &LoadOptions) -> Result<Scene> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(SolidError::Io)?;
        load_ply(&data, options)
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 4];
        let n = reader.read(&mut buf).unwrap_or(0);
        if n >= 4 && &buf[..4] == b"ply\n" { 1.0 }
        else if n >= 3 && &buf[..3] == b"ply" { 0.9 }
        else { 0.0 }
    }
}

fn load_ply(data: &[u8], _options: &LoadOptions) -> Result<Scene> {
    let header = parse_header(data)?;
    let body   = &data[header.header_byte_len..];

    let (vertices, indices) = match header.format {
        PlyFormat::Ascii    => parse_ascii_body(&header.elements, body)?,
        PlyFormat::BinaryLE => parse_binary_body(&header.elements, body, false)?,
        PlyFormat::BinaryBE => parse_binary_body(&header.elements, body, true)?,
    };

    let mut mesh = Mesh::new("PLY Mesh");
    mesh.vertices = vertices;

    if !indices.is_empty() {
        mesh.primitives = vec![Primitive::triangles(indices, None)];
    } else {
        // Point cloud — no face element present.
        let n = mesh.vertices.len() as u32;
        mesh.primitives = vec![Primitive::points((0..n).collect(), None)];
    }

    let mut b = SceneBuilder::named("PLY Scene");
    let mesh_idx = b.push_mesh(mesh);
    let root     = b.add_root_node("Root");
    b.attach_mesh(root, mesh_idx);
    Ok(b.build())
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn find_prop(elem: &Element, names: &[&str]) -> Option<usize> {
    elem.properties.iter().position(|p| {
        names.iter().any(|n| p.name.eq_ignore_ascii_case(n))
    })
}

/// Read a single scalar value from `data[offset..]` in the requested endianness.
fn read_scalar(data: &[u8], offset: usize, ty: ScalarType, be: bool) -> f64 {
    let s = &data[offset..];
    match ty {
        ScalarType::I8  => s[0] as i8 as f64,
        ScalarType::U8  => s[0] as f64,
        ScalarType::I16 => {
            let arr: [u8; 2] = s[..2].try_into().unwrap();
            if be { i16::from_be_bytes(arr) as f64 } else { i16::from_le_bytes(arr) as f64 }
        }
        ScalarType::U16 => {
            let arr: [u8; 2] = s[..2].try_into().unwrap();
            if be { u16::from_be_bytes(arr) as f64 } else { u16::from_le_bytes(arr) as f64 }
        }
        ScalarType::I32 => {
            let arr: [u8; 4] = s[..4].try_into().unwrap();
            if be { i32::from_be_bytes(arr) as f64 } else { i32::from_le_bytes(arr) as f64 }
        }
        ScalarType::U32 => {
            let arr: [u8; 4] = s[..4].try_into().unwrap();
            if be { u32::from_be_bytes(arr) as f64 } else { u32::from_le_bytes(arr) as f64 }
        }
        ScalarType::F32 => {
            let arr: [u8; 4] = s[..4].try_into().unwrap();
            let bits = if be { u32::from_be_bytes(arr) } else { u32::from_le_bytes(arr) };
            f32::from_bits(bits) as f64
        }
        ScalarType::F64 => {
            let arr: [u8; 8] = s[..8].try_into().unwrap();
            let bits = if be { u64::from_be_bytes(arr) } else { u64::from_le_bytes(arr) };
            f64::from_bits(bits)
        }
    }
}

// ── ASCII parser ──────────────────────────────────────────────────────────────

fn parse_ascii_body(elements: &[Element], body: &[u8]) -> Result<(Vec<Vertex>, Vec<u32>)> {
    let text = std::str::from_utf8(body)
        .map_err(|_| SolidError::parse("PLY: body is not valid UTF-8"))?;
    let mut lines = text.lines();

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices:  Vec<u32>    = Vec::new();

    for elem in elements {
        match elem.name.as_str() {
            "vertex" => {
                let xi  = find_prop(elem, &["x"]);
                let yi  = find_prop(elem, &["y"]);
                let zi  = find_prop(elem, &["z"]);
                let nxi = find_prop(elem, &["nx"]);
                let nyi = find_prop(elem, &["ny"]);
                let nzi = find_prop(elem, &["nz"]);
                let ri  = find_prop(elem, &["red", "r"]);
                let gi  = find_prop(elem, &["green", "g"]);
                let bi  = find_prop(elem, &["blue", "b"]);
                let ai  = find_prop(elem, &["alpha", "a"]);

                // UV channels 0–7: channel 0 uses the classic "s"/"t" names.
                let uv_s: [Option<usize>; 8] = [
                    find_prop(elem, &["s", "u", "texture_u"]),
                    find_prop(elem, &["s1", "texture_u1"]),
                    find_prop(elem, &["s2", "texture_u2"]),
                    find_prop(elem, &["s3", "texture_u3"]),
                    find_prop(elem, &["s4", "texture_u4"]),
                    find_prop(elem, &["s5", "texture_u5"]),
                    find_prop(elem, &["s6", "texture_u6"]),
                    find_prop(elem, &["s7", "texture_u7"]),
                ];
                let uv_t: [Option<usize>; 8] = [
                    find_prop(elem, &["t", "v", "texture_v"]),
                    find_prop(elem, &["t1", "texture_v1"]),
                    find_prop(elem, &["t2", "texture_v2"]),
                    find_prop(elem, &["t3", "texture_v3"]),
                    find_prop(elem, &["t4", "texture_v4"]),
                    find_prop(elem, &["t5", "texture_v5"]),
                    find_prop(elem, &["t6", "texture_v6"]),
                    find_prop(elem, &["t7", "texture_v7"]),
                ];

                let color_is_byte = ri.map_or(false, |i| {
                    matches!(elem.properties[i].prop_type, PropType::Scalar(ScalarType::U8))
                });

                for _ in 0..elem.count {
                    let line = lines.next().unwrap_or("");
                    let vals: Vec<f64> = line.split_whitespace()
                        .map(|s| s.parse::<f64>().unwrap_or(0.0))
                        .collect();

                    let get = |idx: Option<usize>| -> f64 {
                        idx.and_then(|i| vals.get(i).copied()).unwrap_or(0.0)
                    };

                    let pos = Vec3::new(get(xi) as f32, get(yi) as f32, get(zi) as f32);
                    let mut v = Vertex::new(pos);

                    if nxi.is_some() && nyi.is_some() && nzi.is_some() {
                        v = v.with_normal(Vec3::new(
                            get(nxi) as f32, get(nyi) as f32, get(nzi) as f32,
                        ));
                    }
                    for ch in 0..8usize {
                        if uv_s[ch].is_some() && uv_t[ch].is_some() {
                            v.uvs[ch] = Some(Vec2::new(
                                get(uv_s[ch]) as f32,
                                get(uv_t[ch]) as f32,
                            ));
                        }
                    }
                    if ri.is_some() {
                        let scale = if color_is_byte { 1.0_f64 / 255.0 } else { 1.0 };
                        let r = (get(ri) * scale) as f32;
                        let g = (get(gi) * scale) as f32;
                        let b = (get(bi) * scale) as f32;
                        let a = if ai.is_some() { (get(ai) * scale) as f32 } else { 1.0 };
                        v = v.with_color(Vec4::new(r, g, b, a));
                    }

                    vertices.push(v);
                }
            }
            "face" => {
                for _ in 0..elem.count {
                    let line = lines.next().unwrap_or("");
                    let nums: Vec<u32> = line.split_whitespace()
                        .filter_map(|s| s.parse::<u32>().ok())
                        .collect();

                    if let Some(&count) = nums.first() {
                        let count = count as usize;
                        if count >= 3 && nums.len() > count {
                            // Fan triangulation: (0, i, i+1) for i in 1..count-1
                            for i in 1..(count - 1) {
                                indices.push(nums[1]);
                                indices.push(nums[1 + i]);
                                indices.push(nums[2 + i]);
                            }
                        }
                    }
                }
            }
            _ => {
                // Skip unknown elements.
                for _ in 0..elem.count {
                    lines.next();
                }
            }
        }
    }

    Ok((vertices, indices))
}

// ── Binary parser ─────────────────────────────────────────────────────────────

fn parse_binary_body(
    elements: &[Element],
    body:     &[u8],
    big_endian: bool,
) -> Result<(Vec<Vertex>, Vec<u32>)> {
    let mut cursor:   usize       = 0;
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices:  Vec<u32>    = Vec::new();

    for elem in elements {
        match elem.name.as_str() {
            "vertex" => {
                let xi  = find_prop(elem, &["x"]);
                let yi  = find_prop(elem, &["y"]);
                let zi  = find_prop(elem, &["z"]);
                let nxi = find_prop(elem, &["nx"]);
                let nyi = find_prop(elem, &["ny"]);
                let nzi = find_prop(elem, &["nz"]);
                let ri  = find_prop(elem, &["red", "r"]);
                let gi  = find_prop(elem, &["green", "g"]);
                let bi  = find_prop(elem, &["blue", "b"]);
                let ai  = find_prop(elem, &["alpha", "a"]);

                let uv_s: [Option<usize>; 8] = [
                    find_prop(elem, &["s", "u", "texture_u"]),
                    find_prop(elem, &["s1", "texture_u1"]),
                    find_prop(elem, &["s2", "texture_u2"]),
                    find_prop(elem, &["s3", "texture_u3"]),
                    find_prop(elem, &["s4", "texture_u4"]),
                    find_prop(elem, &["s5", "texture_u5"]),
                    find_prop(elem, &["s6", "texture_u6"]),
                    find_prop(elem, &["s7", "texture_u7"]),
                ];
                let uv_t: [Option<usize>; 8] = [
                    find_prop(elem, &["t", "v", "texture_v"]),
                    find_prop(elem, &["t1", "texture_v1"]),
                    find_prop(elem, &["t2", "texture_v2"]),
                    find_prop(elem, &["t3", "texture_v3"]),
                    find_prop(elem, &["t4", "texture_v4"]),
                    find_prop(elem, &["t5", "texture_v5"]),
                    find_prop(elem, &["t6", "texture_v6"]),
                    find_prop(elem, &["t7", "texture_v7"]),
                ];

                let color_is_byte = ri.map_or(false, |i| {
                    matches!(elem.properties[i].prop_type, PropType::Scalar(ScalarType::U8))
                });

                for _ in 0..elem.count {
                    let mut prop_vals: Vec<f64> =
                        Vec::with_capacity(elem.properties.len());

                    for prop in &elem.properties {
                        match prop.prop_type {
                            PropType::Scalar(st) => {
                                if cursor + st.byte_size() > body.len() {
                                    return Err(SolidError::parse(
                                        "PLY: unexpected end of binary vertex data",
                                    ));
                                }
                                prop_vals.push(read_scalar(body, cursor, st, big_endian));
                                cursor += st.byte_size();
                            }
                            PropType::List { count_type, value_type } => {
                                if cursor + count_type.byte_size() > body.len() {
                                    return Err(SolidError::parse(
                                        "PLY: unexpected end of binary data",
                                    ));
                                }
                                let cnt = read_scalar(body, cursor, count_type, big_endian)
                                    as usize;
                                cursor += count_type.byte_size();
                                if cursor + cnt * value_type.byte_size() > body.len() {
                                    return Err(SolidError::parse(
                                        "PLY: unexpected end of binary vertex list data",
                                    ));
                                }
                                cursor += cnt * value_type.byte_size();
                                prop_vals.push(0.0); // placeholder to keep index alignment
                            }
                        }
                    }

                    let get = |idx: Option<usize>| -> f64 {
                        idx.and_then(|i| prop_vals.get(i).copied()).unwrap_or(0.0)
                    };

                    let pos = Vec3::new(get(xi) as f32, get(yi) as f32, get(zi) as f32);
                    let mut v = Vertex::new(pos);

                    if nxi.is_some() && nyi.is_some() && nzi.is_some() {
                        v = v.with_normal(Vec3::new(
                            get(nxi) as f32, get(nyi) as f32, get(nzi) as f32,
                        ));
                    }
                    for ch in 0..8usize {
                        if uv_s[ch].is_some() && uv_t[ch].is_some() {
                            v.uvs[ch] = Some(Vec2::new(
                                get(uv_s[ch]) as f32,
                                get(uv_t[ch]) as f32,
                            ));
                        }
                    }
                    if ri.is_some() {
                        let scale = if color_is_byte { 1.0_f64 / 255.0 } else { 1.0 };
                        let r = (get(ri) * scale) as f32;
                        let g = (get(gi) * scale) as f32;
                        let b = (get(bi) * scale) as f32;
                        let a = if ai.is_some() { (get(ai) * scale) as f32 } else { 1.0 };
                        v = v.with_color(Vec4::new(r, g, b, a));
                    }

                    vertices.push(v);
                }
            }
            "face" => {
                let face_prop_names: &[&str] = &["vertex_indices", "vertex_index"];

                for _ in 0..elem.count {
                    let mut face_verts: Vec<u32> = Vec::new();

                    for prop in &elem.properties {
                        match prop.prop_type {
                            PropType::Scalar(st) => {
                                cursor += st.byte_size();
                            }
                            PropType::List { count_type, value_type } => {
                                if cursor + count_type.byte_size() > body.len() {
                                    return Err(SolidError::parse(
                                        "PLY: unexpected end of binary face data",
                                    ));
                                }
                                let cnt = read_scalar(body, cursor, count_type, big_endian)
                                    as usize;
                                cursor += count_type.byte_size();

                                let is_face_list = face_prop_names
                                    .iter()
                                    .any(|n| prop.name.eq_ignore_ascii_case(n));

                                for _ in 0..cnt {
                                    if cursor + value_type.byte_size() > body.len() {
                                        return Err(SolidError::parse(
                                            "PLY: unexpected end of binary face data",
                                        ));
                                    }
                                    let val = read_scalar(body, cursor, value_type, big_endian)
                                        as u32;
                                    cursor += value_type.byte_size();
                                    if is_face_list {
                                        face_verts.push(val);
                                    }
                                }
                            }
                        }
                    }

                    if face_verts.len() >= 3 {
                        for i in 1..(face_verts.len() - 1) {
                            indices.push(face_verts[0]);
                            indices.push(face_verts[i]);
                            indices.push(face_verts[i + 1]);
                        }
                    }
                }
            }
            _ => {
                // Skip unknown elements by consuming their bytes.
                for _ in 0..elem.count {
                    for prop in &elem.properties {
                        match prop.prop_type {
                            PropType::Scalar(st) => {
                                cursor += st.byte_size();
                            }
                            PropType::List { count_type, value_type } => {
                                if cursor + count_type.byte_size() > body.len() {
                                    return Err(SolidError::parse(
                                        "PLY: unexpected end of binary data",
                                    ));
                                }
                                let cnt = read_scalar(body, cursor, count_type, big_endian)
                                    as usize;
                                cursor += count_type.byte_size();
                                cursor += cnt * value_type.byte_size();
                            }
                        }
                    }
                }
            }
        }
    }

    Ok((vertices, indices))
}
