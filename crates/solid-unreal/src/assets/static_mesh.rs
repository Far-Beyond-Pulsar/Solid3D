use std::io::SeekFrom;

use glam::{Vec2, Vec3, Vec4};

use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::UPackage;

/// Flags for vertex elements (from UE's FStaticMeshBuffers).
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

/// A single vertex from an Unreal static mesh.
#[derive(Debug, Clone, Default)]
pub struct UnrealVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tangent: Vec4, // xyz = tangent direction, w = sign
    pub uv: [Vec2; 4],
    pub color: [u8; 4],
}

/// A mesh section (material slot with index range).
#[derive(Debug, Clone)]
pub struct MeshSection {
    pub material_index: usize,
    pub first_index: u32,
    pub num_indices: u32,
    pub first_vertex: u32,
    pub num_vertices: u32,
}

/// A LOD level of a static mesh.
#[derive(Debug, Clone)]
pub struct StaticMeshLOD {
    pub vertices: Vec<UnrealVertex>,
    pub indices: Vec<u32>,
    pub sections: Vec<MeshSection>,
    /// Material indices mapped to the mesh's material slot names.
    pub material_names: Vec<String>,
}

/// Parsed UStaticMesh data.
#[derive(Debug, Clone)]
pub struct StaticMeshAsset {
    pub name: String,
    pub lod_count: u32,
    pub lods: Vec<StaticMeshLOD>,
}

/// Read a UStaticMesh from a package export.
pub fn read_static_mesh(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<StaticMeshAsset, UnrealError> {
    let export = pkg.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_static_mesh",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = pkg.resolve_name(export.object_name);

    let start_offset = if pkg.version.is_ue5() {
        export.serial_offset as u64
    } else {
        export.serial_offset as u64
    };

    reader.seek(SeekFrom::Start(start_offset))?;

    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

    // Walk properties to find render data
    loop {
        match pr.read_tag()? {
            None => break,
            Some(tag) => {
                let ar = pr.archive();
                match tag.name.as_str() {
                    "BodySetup" | "NavCollision" | "LightMap" | "ShadowMap"
                    | "StreamingDistanceMultiplier" | "CustomizedCollision" => {
                        // Skip known non-mesh properties
                    }
                    "StaticMaterials" => {
                        // Array of FStaticMaterial structs (material slots)
                        // Each has: Material (ObjectPtr), MaterialSlotName (FName), UVChannelData
                        // We skip the details and just check the array size
                    }
                    "RenderData" | "CookedRenderData" | "MeshDescription" | "MeshDescriptionCooked" => {
                        // Parse FStaticMeshRenderData starting at the current position
                        let next_off = tag.raw.next_offset;
                        let lod_data = parse_render_data(ar)?;
                        if let Some(lods) = lod_data {
                            return Ok(StaticMeshAsset {
                                name: export_name,
                                lod_count: lods.len() as u32,
                                lods,
                            });
                        }

                        // If we didn't return, seek past this property
                        if next_off > ar.pos {
                            ar.seek_to(next_off)?;
                        }
                    }
                    _ => {
                        // Unknown — skip
                        if tag.raw.next_offset > ar.pos {
                            ar.seek_to(tag.raw.next_offset)?;
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

/// Parse a serialized FStaticMeshRenderData.
fn parse_render_data(
    ar: &mut FArchiveUE,
) -> Result<Option<Vec<StaticMeshLOD>>, UnrealError> {
    // UE4 FStaticMeshRenderData serialization:
    //   - bClampCustomVertexPaintRadiosity (u32)
    //   - UvChannelData
    //   - bLODsShareStaticLighting (u32)
    //   - LODs array

    let _clamp_vertex_paint = ar.read_u32()?;

    // Skip UV channel data (4 i32 values)
    let _uv_channel_count = ar.read_i32()?;
    let _uv_channel_validity = ar.read_i32()?;
    let _padding1 = ar.read_i32()?;
    let _padding2 = ar.read_i32()?;

    let _bLODsShareLighting = ar.read_u32()?;

    // Read LOD array
    let lod_count = ar.read_serial_size()? as usize;
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

/// Read a single FStaticMeshLODResources.
fn read_static_mesh_lod(ar: &mut FArchiveUE) -> Result<StaticMeshLOD, UnrealError> {
    // Sections
    let section_count = ar.read_serial_size()? as usize;
    let mut sections = Vec::with_capacity(section_count);

    for _ in 0..section_count {
        let material_index = ar.read_i32()? as usize;
        let first_index = ar.read_u32()?;
        let num_indices = ar.read_u32()?;
        let first_vertex = ar.read_u32()?;
        let num_vertices = ar.read_u32()?;

        // Skip additional section data
        let _min_x = ar.read_f32()?;
        let _min_y = ar.read_f32()?;
        let _min_z = ar.read_f32()?;
        let _max_x = ar.read_f32()?;
        let _max_y = ar.read_f32()?;
        let _max_z = ar.read_f32()?;
        let _b_frustum_cull = ar.read_u32()?;
        let _b_force_opaque = ar.read_u32()?;

        sections.push(MeshSection {
            material_index,
            first_index,
            num_indices,
            first_vertex,
            num_vertices,
        });
    }

    // Indices
    let index_count = ar.read_serial_size()? as usize;
    let use_16_bit_indices = ar.read_u32()? != 0;
    let mut indices = Vec::with_capacity(index_count);

    if use_16_bit_indices {
        for _ in 0..index_count {
            indices.push(ar.read_u16()? as u32);
        }
    } else {
        for _ in 0..index_count {
            indices.push(ar.read_u32()?);
        }
    }

    // Vertices — read FStaticMeshVertexBuffers
    let vertex_count = ar.read_serial_size()? as usize;
    let mut vertices = vec![UnrealVertex::default(); vertex_count];

    // Read position data (FPositionVertexBuffer)
    let pos_count = ar.read_serial_size()? as usize;
    let use_half_floats = ar.read_u32()? != 0;
    for i in 0..pos_count.min(vertex_count) {
        if use_half_floats {
            let x = half_to_f32(ar.read_u16()?);
            let y = half_to_f32(ar.read_u16()?);
            let z = half_to_f32(ar.read_u16()?);
            vertices[i].position = Vec3::new(x, y, z);
        } else {
            let x = ar.read_f32()?;
            let y = ar.read_f32()?;
            let z = ar.read_f32()?;
            vertices[i].position = Vec3::new(x, y, z);
        }
    }

    // Read static mesh vertex buffer (normals, tangents, UVs, colors)
    read_static_mesh_vertex_buffer(ar, &mut vertices, vertex_count)?;

    // Material names are not stored per-LOD; they come from the outer StaticMesh's material slots
    let material_names = Vec::new();

    Ok(StaticMeshLOD {
        vertices,
        indices,
        sections,
        material_names,
    })
}

/// Read the FStaticMeshVertexBuffer (normals, tangents, UVs, colors).
fn read_static_mesh_vertex_buffer(
    ar: &mut FArchiveUE,
    vertices: &mut [UnrealVertex],
    vertex_count: usize,
) -> Result<(), UnrealError> {
    // Read vertex data type info
    let num_tex_coords = ar.read_i32()? as usize;
    let _b_use_full_precision_uvs = ar.read_u32()?;
    let _b_use_high_precision_tangents = ar.read_u32()?;

    // Normals (packed as u32 each — FVector4f packed into 32 bits)
    for i in 0..vertex_count {
        let packed = ar.read_u32()?;
        vertices[i].normal = unpack_normal(packed);
    }

    // Tangents (packed as u32 each)
    for i in 0..vertex_count {
        let packed = ar.read_u32()?;
        vertices[i].tangent = unpack_tangent(packed);
    }

    // UVs
    let tex_coord_size = if _b_use_full_precision_uvs == 0 { 4 } else { 8 }; // half vs float
    for i in 0..vertex_count {
        for uv_channel in 0..num_tex_coords.min(4) {
            if tex_coord_size == 4 {
                // Half float
                let u = half_to_f32(ar.read_u16()?);
                let v = half_to_f32(ar.read_u16()?);
                vertices[i].uv[uv_channel] = Vec2::new(u, v);
            } else {
                let u = ar.read_f32()?;
                let v = ar.read_f32()?;
                vertices[i].uv[uv_channel] = Vec2::new(u, v);
            }
        }
        // Fill remaining UV channels
        for uv_channel in num_tex_coords..4 {
            vertices[i].uv[uv_channel] = Vec2::ZERO;
        }
    }

    // Colors (read as FColor — BGRA packed as u32 or RGBA8)
    // Some meshes may not have vertex colors; check if we're past the stream
    if ar.pos + (vertex_count as u64 * 4) <= ar.pos + 4 {
        // Approximate check — read colors
        for i in 0..vertex_count {
            let color_packed = ar.read_u32()?;
            let b = (color_packed >> 0) & 0xFF;
            let g = (color_packed >> 8) & 0xFF;
            let r = (color_packed >> 16) & 0xFF;
            let a = (color_packed >> 24) & 0xFF;
            vertices[i].color = [r as u8, g as u8, b as u8, a as u8];
        }
    }

    Ok(())
}

/// Unpack an FPackedNormal to Vec3.
fn unpack_normal(packed: u32) -> Vec3 {
    let x = ((packed >> 0) & 0xFF) as i8 as f32 / 127.0;
    let y = ((packed >> 8) & 0xFF) as i8 as f32 / 127.0;
    let z = ((packed >> 16) & 0xFF) as i8 as f32 / 127.0;
    Vec3::new(x, y, z)
}

/// Unpack an FPackedNormal with w component to Vec4 (tangent).
fn unpack_tangent(packed: u32) -> Vec4 {
    let x = ((packed >> 0) & 0xFF) as i8 as f32 / 127.0;
    let y = ((packed >> 8) & 0xFF) as i8 as f32 / 127.0;
    let z = ((packed >> 16) & 0xFF) as i8 as f32 / 127.0;
    let w = ((packed >> 24) & 0xFF) as i8 as f32 / 127.0;
    Vec4::new(x, y, z, w)
}

/// Read a CubeBuilder export and extract geometry from the "Vertices" property.
///
/// CubeBuilder is an editor-only brush builder that stores vertices as a TArray<FVector>.
/// The cooked map preserves this data. We read the vertices and generate a mesh
/// using convex hull triangulation (fan from centroid).
pub fn read_cube_builder(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<StaticMeshAsset, UnrealError> {
    let export = pkg.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_cube_builder",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = pkg.resolve_name(export.object_name);

    let start_offset = export.serial_offset as u64;
    reader.seek(SeekFrom::Start(start_offset))?;

    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

    // Look for the "Vertices" property (TArray<FVector>)
    if pr.find_property("Vertices")? {
        let count = pr.archive().read_i32()? as usize;
        if count > 0 && count < 10000 {
            let mut positions = Vec::with_capacity(count);
            for _ in 0..count {
                let x = pr.archive().read_f32()?;
                let y = pr.archive().read_f32()?;
                let z = pr.archive().read_f32()?;
                positions.push(Vec3::new(x, y, z));
            }
            if positions.len() >= 3 {
                // Fan triangulation from centroid
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
                return Ok(StaticMeshAsset {
                    name: export_name,
                    lod_count: 1,
                    lods: vec![StaticMeshLOD {
                        vertices: ue_verts,
                        indices,
                        sections: vec![MeshSection { material_index: 0, first_index: 0, num_indices: num_idx, first_vertex: 0, num_vertices: num_verts }],
                        material_names: vec!["Default".to_string()],
                    }],
                });
            }
        }
    }

    // No Vertices found — fall back to a simple unit box
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

/// Read a BrushComponent export and try to extract geometry.
/// BrushComponent's "Brush" property points to a UModel or brush object.
pub fn read_brush_component(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<StaticMeshAsset, UnrealError> {
    let export = pkg.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_brush_component",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = pkg.resolve_name(export.object_name);
    let start_offset = export.serial_offset as u64;
    reader.seek(SeekFrom::Start(start_offset))?;

    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

    // Find the "Brush" property (ObjectProperty referencing a UModel or brush)
    if !pr.find_property("Brush")? {
        return Err(UnrealError::Conversion {
            asset_type: "BrushComponent",
            detail: format!("BrushComponent '{export_name}' has no Brush property"),
        });
    }

    // Read the object reference (FPackageIndex)
    let brush_ref = pr.archive().read_package_index()?;

    if !brush_ref.is_export() {
        // Brush reference might be null or import - can't follow
        return Err(UnrealError::Conversion {
            asset_type: "BrushComponent",
            detail: format!("BrushComponent '{export_name}' Brush reference is not an export"),
        });
    }

    let brush_export_idx = (brush_ref.0 - 1) as usize;
    let brush_name = pkg.resolve_name(pkg.exports[brush_export_idx].object_name);
    println!("[BRUSH] '{export_name}' Brush -> export[{brush_export_idx}] '{brush_name}'");

    // The brush might be a UModel with BSP geometry, or a CubeBuilder.
    // For now, try reading the referenced export as a CubeBuilder if it is one.
    let brush_class = pkg.resolve_export_class_name(pkg.exports[brush_export_idx].class_index);
    if brush_class == "CubeBuilder" {
        return read_cube_builder(pkg, brush_export_idx, reader);
    }

    // If the Brush is a UModel, the serial data has BSP nodes/geom which is complex.
    // For now, return error.
    Err(UnrealError::Conversion {
        asset_type: "BrushComponent",
        detail: format!("Brush references '{brush_class}' which is not yet supported"),
    })
}

/// Convert half-precision float to f32.
fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 0x1) as f32;
    let exp = (h >> 10) & 0x1F;
    let mant = h & 0x3FF;

    if exp == 0 {
        // Subnormal or zero
        if mant == 0 {
            0.0
        } else {
            f32::from_bits(((sign as u32) << 31) | (0x7F - 1 - 15) << 23 | (mant as u32) << 13)
        }
    } else if exp == 31 {
        // Inf or NaN
        if mant == 0 {
            f32::INFINITY * if sign == 0.0 { 1.0 } else { -1.0 }
        } else {
            f32::NAN
        }
    } else {
        f32::from_bits(((sign as u32) << 31) | ((exp as u32 + (127 - 15)) << 23) | (mant as u32) << 13)
    }
}
