use std::io::{Read, Seek, SeekFrom};

use glam::{Vec2, Vec3, Vec4};

use crate::error::UnrealError;
use crate::reader;
use crate::uobject::property::PropertyReader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexAttributeType {
    Position,
    Normal,
    Tangent,
    UV0,
    UV1,
    UV2,
    UV3,
    Color,
}

#[derive(Debug, Clone, Default)]
pub struct UnrealVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tangent: Vec4,
    pub uv: [Vec2; 4],
    pub color: [u8; 4],
}

#[derive(Debug, Clone)]
pub struct MeshSection {
    pub material_index: usize,
    pub first_index: u32,
    pub num_indices: u32,
    pub first_vertex: u32,
    pub num_vertices: u32,
}

#[derive(Debug, Clone)]
pub struct StaticMeshLOD {
    pub vertices: Vec<UnrealVertex>,
    pub indices: Vec<u32>,
    pub sections: Vec<MeshSection>,
    pub material_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StaticMeshAsset {
    pub name: String,
    pub lod_count: u32,
    pub lods: Vec<StaticMeshLOD>,
}

pub fn read_static_mesh(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    export_idx: usize,
) -> Result<StaticMeshAsset, UnrealError> {
    let export = header.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_static_mesh",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();

    let start_offset = export.serial_offset as u64;
    header.archive.seek(SeekFrom::Start(start_offset))?;

    let pr = PropertyReader::new(&header.names);

    loop {
        match pr.read_tag(&mut header.archive)? {
            None => break,
            Some(tag) => {
                match tag.name.as_str() {
                    "BodySetup" | "NavCollision" | "LightMap" | "ShadowMap"
                    | "StreamingDistanceMultiplier" | "CustomizedCollision" => {}
                    "StaticMaterials" => {}
                    "RenderData" | "CookedRenderData" | "MeshDescription" | "MeshDescriptionCooked" => {
                        let next_off = tag.raw.next_offset;
                        let lod_data = parse_render_data(&mut header.archive)?;
                        if let Some(lods) = lod_data {
                            return Ok(StaticMeshAsset {
                                name: export_name,
                                lod_count: lods.len() as u32,
                                lods,
                            });
                        }
                        if next_off > tag.raw.next_offset {
                            header.archive.seek(SeekFrom::Start(next_off))?;
                        }
                    }
                    _ => {
                        let next = tag.raw.next_offset;
                        let pos = match header.archive.stream_position() {
                            Ok(p) => p,
                            Err(_) => 0,
                        };
                        if next > pos {
                            header.archive.seek(SeekFrom::Start(next))?;
                        }
                    }
                }
            }
        }
    }

    Err(UnrealError::Conversion {
        asset_type: "StaticMesh",
        detail: format!("mesh '{}' has no render data", export_name),
    })
}

fn parse_render_data(
    ar: &mut uasset::Archive<std::io::Cursor<&[u8]>>,
) -> Result<Option<Vec<StaticMeshLOD>>, UnrealError> {
    let _clamp_vertex_paint = reader::read_u32(ar)?;
    let _uv_channel_count = reader::read_i32(ar)?;
    let _uv_channel_validity = reader::read_i32(ar)?;
    let _padding1 = reader::read_i32(ar)?;
    let _padding2 = reader::read_i32(ar)?;

    let _bLODsShareLighting = reader::read_u32(ar)?;

    let lod_count = reader::read_i32(ar)? as usize;
    let mut lods = Vec::with_capacity(lod_count);

    for _ in 0..lod_count {
        let lod = read_static_mesh_lod(ar)?;
        lods.push(lod);
    }

    if lods.is_empty() {
        return Ok(None);
    }

    Ok(Some(lods))
}

fn read_static_mesh_lod(
    ar: &mut uasset::Archive<std::io::Cursor<&[u8]>>,
) -> Result<StaticMeshLOD, UnrealError> {
    let section_count = reader::read_i32(ar)? as usize;
    let mut sections = Vec::with_capacity(section_count);

    for _ in 0..section_count {
        let material_index = reader::read_i32(ar)? as usize;
        let first_index = reader::read_u32(ar)?;
        let num_indices = reader::read_u32(ar)?;
        let first_vertex = reader::read_u32(ar)?;
        let num_vertices = reader::read_u32(ar)?;

        let _min_x = reader::read_f32(ar)?;
        let _min_y = reader::read_f32(ar)?;
        let _min_z = reader::read_f32(ar)?;
        let _max_x = reader::read_f32(ar)?;
        let _max_y = reader::read_f32(ar)?;
        let _max_z = reader::read_f32(ar)?;
        let _b_frustum_cull = reader::read_u32(ar)?;
        let _b_force_opaque = reader::read_u32(ar)?;

        sections.push(MeshSection {
            material_index,
            first_index,
            num_indices,
            first_vertex,
            num_vertices,
        });
    }

    let index_count = reader::read_i32(ar)? as usize;
    let use_16_bit_indices = reader::read_u32(ar)? != 0;
    let mut indices = Vec::with_capacity(index_count);

    if use_16_bit_indices {
        for _ in 0..index_count {
            indices.push(reader::read_u16(ar)? as u32);
        }
    } else {
        for _ in 0..index_count {
            indices.push(reader::read_u32(ar)?);
        }
    }

    let vertex_count = reader::read_i32(ar)? as usize;
    let mut vertices = vec![UnrealVertex::default(); vertex_count];

    let pos_count = reader::read_i32(ar)? as usize;
    let use_half_floats = reader::read_u32(ar)? != 0;
    for i in 0..pos_count.min(vertex_count) {
        if use_half_floats {
            let x = half_to_f32(reader::read_u16(ar)?);
            let y = half_to_f32(reader::read_u16(ar)?);
            let z = half_to_f32(reader::read_u16(ar)?);
            vertices[i].position = Vec3::new(x, y, z);
        } else {
            let x = reader::read_f32(ar)?;
            let y = reader::read_f32(ar)?;
            let z = reader::read_f32(ar)?;
            vertices[i].position = Vec3::new(x, y, z);
        }
    }

    read_static_mesh_vertex_buffer(ar, &mut vertices, vertex_count)?;

    let material_names = Vec::new();

    Ok(StaticMeshLOD {
        vertices,
        indices,
        sections,
        material_names,
    })
}

fn read_static_mesh_vertex_buffer(
    ar: &mut uasset::Archive<std::io::Cursor<&[u8]>>,
    vertices: &mut [UnrealVertex],
    vertex_count: usize,
) -> Result<(), UnrealError> {
    let num_tex_coords = reader::read_i32(ar)? as usize;
    let _b_use_full_precision_uvs = reader::read_u32(ar)?;
    let _b_use_high_precision_tangents = reader::read_u32(ar)?;

    for i in 0..vertex_count {
        let packed = reader::read_u32(ar)?;
        vertices[i].normal = unpack_normal(packed);
    }

    for i in 0..vertex_count {
        let packed = reader::read_u32(ar)?;
        vertices[i].tangent = unpack_tangent(packed);
    }

    let tex_coord_size = if _b_use_full_precision_uvs == 0 { 4 } else { 8 };
    for i in 0..vertex_count {
        for uv_channel in 0..num_tex_coords.min(4) {
            if tex_coord_size == 4 {
                let u = half_to_f32(reader::read_u16(ar)?);
                let v = half_to_f32(reader::read_u16(ar)?);
                vertices[i].uv[uv_channel] = Vec2::new(u, v);
            } else {
                let u = reader::read_f32(ar)?;
                let v = reader::read_f32(ar)?;
                vertices[i].uv[uv_channel] = Vec2::new(u, v);
            }
        }
        for uv_channel in num_tex_coords..4 {
            vertices[i].uv[uv_channel] = Vec2::ZERO;
        }
    }

    if let Ok(pos) = ar.stream_position() {
        let _ = pos;
        for i in 0..vertex_count {
            let color_packed = reader::read_u32(ar)?;
            let b = (color_packed >> 0) & 0xFF;
            let g = (color_packed >> 8) & 0xFF;
            let r = (color_packed >> 16) & 0xFF;
            let a = (color_packed >> 24) & 0xFF;
            vertices[i].color = [r as u8, g as u8, b as u8, a as u8];
        }
    }

    Ok(())
}

fn unpack_normal(packed: u32) -> Vec3 {
    let x = ((packed >> 0) & 0xFF) as i8 as f32 / 127.0;
    let y = ((packed >> 8) & 0xFF) as i8 as f32 / 127.0;
    let z = ((packed >> 16) & 0xFF) as i8 as f32 / 127.0;
    Vec3::new(x, y, z)
}

fn unpack_tangent(packed: u32) -> Vec4 {
    let x = ((packed >> 0) & 0xFF) as i8 as f32 / 127.0;
    let y = ((packed >> 8) & 0xFF) as i8 as f32 / 127.0;
    let z = ((packed >> 16) & 0xFF) as i8 as f32 / 127.0;
    let w = ((packed >> 24) & 0xFF) as i8 as f32 / 127.0;
    Vec4::new(x, y, z, w)
}

pub fn read_cube_builder(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    export_idx: usize,
) -> Result<StaticMeshAsset, UnrealError> {
    let export = header.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_cube_builder",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();

    let start_offset = export.serial_offset as u64;
    let data_len = export.serial_size as usize;

    header.archive.seek(SeekFrom::Start(start_offset))?;
    let mut raw_data = vec![0u8; data_len];
    header.archive.reader.read_exact(&mut raw_data).ok();

    let mut best_positions: Vec<Vec3> = Vec::new();

    for start_byte in (32..raw_data.len().saturating_sub(12)).step_by(1) {
        for stride in &[12usize, 24] {
            let mut positions = Vec::new();
            let mut valid = true;
            let count_max = (raw_data.len() - start_byte) / stride;
            if count_max < 3 || count_max > 500 { continue; }

            for i in 0..count_max {
                let off = start_byte + i * stride;
                let (x, y, z) = if *stride == 12 {
                    (f32::from_le_bytes(raw_data[off..off+4].try_into().unwrap()),
                     f32::from_le_bytes(raw_data[off+4..off+8].try_into().unwrap()),
                     f32::from_le_bytes(raw_data[off+8..off+12].try_into().unwrap()))
                } else {
                    (f64::from_le_bytes(raw_data[off..off+8].try_into().unwrap()) as f32,
                     f64::from_le_bytes(raw_data[off+8..off+16].try_into().unwrap()) as f32,
                     f64::from_le_bytes(raw_data[off+16..off+24].try_into().unwrap()) as f32)
                };
                if !x.is_finite() || !y.is_finite() || !z.is_finite() ||
                   x.abs() > 1e6 || y.abs() > 1e6 || z.abs() > 1e6 { valid = false; break; }
                positions.push(Vec3::new(x, y, z));
            }

            if valid && positions.len() > best_positions.len() {
                let min_x = positions.iter().map(|p| p.x).fold(f32::MAX, f32::min);
                let max_x = positions.iter().map(|p| p.x).fold(f32::MIN, f32::max);
                let range = (max_x - min_x).abs();
                if range > 1.0 && range < 100000.0 {
                    best_positions = positions;
                }
            }
        }
    }

    if best_positions.len() >= 3 {
        return build_mesh_from_positions(export_name, best_positions);
    }

    let box_verts = [
        Vec3::new(-50.0, -50.0, -50.0), Vec3::new(50.0, -50.0, -50.0),
        Vec3::new(50.0, 50.0, -50.0), Vec3::new(-50.0, 50.0, -50.0),
        Vec3::new(-50.0, -50.0, 50.0), Vec3::new(50.0, -50.0, 50.0),
        Vec3::new(50.0, 50.0, 50.0), Vec3::new(-50.0, 50.0, 50.0),
    ];
    let faces: [[u32; 4]; 6] = [
        [0, 1, 2, 3], [1, 5, 6, 2], [5, 4, 7, 6],
        [4, 0, 3, 7], [3, 2, 6, 7], [4, 5, 1, 0],
    ];
    let ue_verts: Vec<UnrealVertex> = box_verts.iter().map(|p| {
        UnrealVertex { position: *p, ..Default::default() }
    }).collect();
    let mut indices = Vec::new();
    for face in &faces {
        indices.push(face[0]); indices.push(face[1]); indices.push(face[2]);
        indices.push(face[0]); indices.push(face[2]); indices.push(face[3]);
    }
    let num_verts = ue_verts.len() as u32;
    let num_idx = indices.len() as u32;
    Ok(StaticMeshAsset {
        name: export_name,
        lod_count: 1,
        lods: vec![StaticMeshLOD {
            vertices: ue_verts,
            indices,
            sections: vec![MeshSection { material_index: 0, first_index: 0, num_indices: num_idx, first_vertex: 0, num_vertices: num_verts }],
            material_names: vec!["Default".to_string()],
        }],
    })
}

fn build_mesh_from_positions(export_name: String, positions: Vec<Vec3>) -> Result<StaticMeshAsset, UnrealError> {
    let centroid = positions.iter().sum::<Vec3>() / positions.len() as f32;
    let ue_verts: Vec<UnrealVertex> = positions.iter().map(|p| {
        UnrealVertex { position: *p, ..Default::default() }
    }).collect();
    let mut sorted: Vec<(usize, f32)> = positions.iter().enumerate().map(|(i, p)| {
        let dir = (*p - centroid).normalize_or_zero();
        (i, f32::atan2(dir.z, dir.x))
    }).collect();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut indices = Vec::new();
    for i in 0..sorted.len() {
        let a = sorted[i].0 as u32;
        let b = sorted[(i + 1) % sorted.len()].0 as u32;
        indices.push(a); indices.push(b); indices.push(sorted[0].0 as u32);
    }
    let num_verts = ue_verts.len() as u32;
    let num_idx = indices.len() as u32;
    Ok(StaticMeshAsset {
        name: export_name,
        lod_count: 1,
        lods: vec![StaticMeshLOD {
            vertices: ue_verts,
            indices,
            sections: vec![MeshSection { material_index: 0, first_index: 0, num_indices: num_idx, first_vertex: 0, num_vertices: num_verts }],
            material_names: vec!["Default".to_string()],
        }],
    })
}

pub fn read_brush_component(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    export_idx: usize,
) -> Result<StaticMeshAsset, UnrealError> {
    let export = header.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_brush_component",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();
    let start_offset = export.serial_offset as u64;
    header.archive.seek(SeekFrom::Start(start_offset))?;

    let pr = PropertyReader::new(&header.names);

    if !pr.find_property(&mut header.archive, "Brush")? {
        return Err(UnrealError::Conversion {
            asset_type: "BrushComponent",
            detail: format!("BrushComponent '{export_name}' has no Brush property"),
        });
    }

    let brush_ref = reader::read_package_index(&mut header.archive)?;

    if brush_ref > 0 {
        let brush_export_idx = (brush_ref - 1) as usize;
        if let Some(brush_export) = header.exports.get(brush_export_idx) {
            let brush_class = match brush_export.class() {
                uasset::ObjectReference::Import { import_index } => {
                    header.imports.get(import_index)
                        .and_then(|i| header.resolve_name(&i.object_name).ok())
                        .map(|c| c.to_string()).unwrap_or_default()
                }
                uasset::ObjectReference::Export { export_index } => {
                    header.exports.get(export_index)
                        .and_then(|e| header.resolve_name(&e.object_name).ok())
                        .map(|c| c.to_string()).unwrap_or_default()
                }
                uasset::ObjectReference::None => String::new(),
            };
            if brush_class == "CubeBuilder" {
                return read_cube_builder(header, brush_export_idx);
            }
        }
    }

    Err(UnrealError::Conversion {
        asset_type: "BrushComponent",
        detail: format!("BrushComponent '{export_name}' Brush ref={:?}", brush_ref),
    })
}

fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 0x1) as f32;
    let exp = (h >> 10) & 0x1F;
    let mant = h & 0x3FF;

    if exp == 0 {
        if mant == 0 {
            0.0
        } else {
            f32::from_bits(((sign as u32) << 31) | (0x7F - 1 - 15) << 23 | (mant as u32) << 13)
        }
    } else if exp == 31 {
        if mant == 0 {
            f32::INFINITY * if sign == 0.0 { 1.0 } else { -1.0 }
        } else {
            f32::NAN
        }
    } else {
        f32::from_bits(((sign as u32) << 31) | ((exp as u32 + (127 - 15)) << 23) | (mant as u32) << 13)
    }
}
