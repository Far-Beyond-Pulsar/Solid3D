//! Buffer resolution and typed accessor reads.

use crate::document::{GltfAccessor, GltfRoot, component_size, num_components};
use solid_rs::error::{Result, SolidError};
use std::path::Path;

/// Resolve all buffer URIs into raw byte vectors.
/// `bin_chunk` is the GLB binary chunk (may be empty for .gltf files).
pub fn resolve_buffers(
    root: &GltfRoot,
    bin_chunk: &[u8],
    base_dir: Option<&Path>,
) -> Result<Vec<Vec<u8>>> {
    root.buffers
        .iter()
        .enumerate()
        .map(|(i, buf)| {
            if let Some(uri) = &buf.uri {
                if let Some(b64) = uri.strip_prefix("data:") {
                    let comma = b64.find(',').ok_or_else(|| {
                        SolidError::parse("glTF buffer data URI missing comma")
                    })?;
                    let encoded = &b64[comma + 1..];
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .map_err(|e| SolidError::parse(format!("base64 decode: {e}")))
                } else {
                    let path = base_dir
                        .map(|d| d.join(uri))
                        .unwrap_or_else(|| Path::new(uri).to_path_buf());
                    std::fs::read(&path).map_err(|e| {
                        SolidError::parse(format!("reading buffer {}: {}", path.display(), e))
                    })
                }
            } else if i == 0 && !bin_chunk.is_empty() {
                Ok(bin_chunk.to_vec())
            } else {
                Err(SolidError::parse(format!("glTF buffer {i} has no URI and no binary chunk")))
            }
        })
        .collect()
}

fn get_slice<'a>(
    root: &GltfRoot,
    buffers: &'a [Vec<u8>],
    acc: &GltfAccessor,
) -> Result<(&'a [u8], usize)> {
    let bv_idx = acc.buffer_view.ok_or_else(|| SolidError::parse("accessor missing bufferView"))?;
    let bv = &root.buffer_views[bv_idx];
    let buf = &buffers[bv.buffer];
    let start = bv.byte_offset + acc.byte_offset;
    let n_comps = num_components(&acc.type_);
    let comp_sz = component_size(acc.component_type);
    let stride = bv.byte_stride.unwrap_or(n_comps * comp_sz);
    let slice = &buf[start..];
    Ok((slice, stride))
}

/// Read an accessor as f32 values. Handles FLOAT, normalized UNSIGNED_BYTE/SHORT.
/// Returns a flat vec of length `accessor.count * num_components(accessor.type_)`.
pub fn read_f32(root: &GltfRoot, buffers: &[Vec<u8>], acc_idx: usize) -> Result<Vec<f32>> {
    let acc = &root.accessors[acc_idx];
    let n_comps = num_components(&acc.type_);
    let (slice, stride) = get_slice(root, buffers, acc)?;
    let comp_sz = component_size(acc.component_type);
    let mut out = Vec::with_capacity(acc.count * n_comps);
    for i in 0..acc.count {
        let base = i * stride;
        for c in 0..n_comps {
            let off = base + c * comp_sz;
            let val = match acc.component_type {
                5126 => f32::from_le_bytes(slice[off..off+4].try_into().unwrap()),
                5123 if acc.normalized => {
                    let v = u16::from_le_bytes(slice[off..off+2].try_into().unwrap());
                    v as f32 / 65535.0
                }
                5121 if acc.normalized => slice[off] as f32 / 255.0,
                5121 => slice[off] as f32,
                5123 => u16::from_le_bytes(slice[off..off+2].try_into().unwrap()) as f32,
                5120 => slice[off] as i8 as f32,
                5122 => i16::from_le_bytes(slice[off..off+2].try_into().unwrap()) as f32,
                5125 => u32::from_le_bytes(slice[off..off+4].try_into().unwrap()) as f32,
                _ => 0.0,
            };
            out.push(val);
        }
    }
    Ok(out)
}

/// Read an accessor as u32 indices. Handles UNSIGNED_BYTE/SHORT/INT.
pub fn read_u32(root: &GltfRoot, buffers: &[Vec<u8>], acc_idx: usize) -> Result<Vec<u32>> {
    let acc = &root.accessors[acc_idx];
    let (slice, stride) = get_slice(root, buffers, acc)?;
    let _comp_sz = component_size(acc.component_type);
    let mut out = Vec::with_capacity(acc.count);
    for i in 0..acc.count {
        let off = i * stride;
        let v = match acc.component_type {
            5121 => slice[off] as u32,
            5123 => u16::from_le_bytes(slice[off..off+2].try_into().unwrap()) as u32,
            5125 => u32::from_le_bytes(slice[off..off+4].try_into().unwrap()),
            _ => 0,
        };
        out.push(v);
    }
    Ok(out)
}

/// Read u16 values (used for JOINTS_0).
pub fn read_u16_vec4(root: &GltfRoot, buffers: &[Vec<u8>], acc_idx: usize) -> Result<Vec<[u16; 4]>> {
    let acc = &root.accessors[acc_idx];
    let (slice, stride) = get_slice(root, buffers, acc)?;
    let comp_sz = component_size(acc.component_type);
    let mut out = Vec::with_capacity(acc.count);
    for i in 0..acc.count {
        let base = i * stride;
        let mut joints = [0u16; 4];
        for c in 0..4 {
            let off = base + c * comp_sz;
            joints[c] = match acc.component_type {
                5121 => slice[off] as u16,
                5123 => u16::from_le_bytes(slice[off..off+2].try_into().unwrap()),
                _ => 0,
            };
        }
        out.push(joints);
    }
    Ok(out)
}
