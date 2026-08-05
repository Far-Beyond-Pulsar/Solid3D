//! Buffer resolution and typed accessor reads.

use crate::document::{component_size, num_components, GltfAccessor, GltfRoot};
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
                    let comma = b64
                        .find(',')
                        .ok_or_else(|| SolidError::parse("glTF buffer data URI missing comma"))?;
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
                Err(SolidError::parse(format!(
                    "glTF buffer {i} has no URI and no binary chunk"
                )))
            }
        })
        .collect()
}

/// Maximum total bytes a single accessor may declare (guards against hostile
/// `count` values before any allocation happens).
const MAX_ACCESSOR_BYTES: usize = 1 << 30;

/// Resolve the byte slice backing an accessor, validating every index and
/// offset read from the file.
fn get_slice<'a>(
    root: &GltfRoot,
    buffers: &'a [Vec<u8>],
    acc: &GltfAccessor,
) -> Result<(&'a [u8], usize)> {
    let bv_idx = acc
        .buffer_view
        .ok_or_else(|| SolidError::parse("accessor missing bufferView"))?;
    if bv_idx >= root.buffer_views.len() {
        return Err(SolidError::parse(format!(
            "glTF: accessor references bufferView {bv_idx}, but only {} exist",
            root.buffer_views.len()
        )));
    }
    let bv = &root.buffer_views[bv_idx];
    if bv.buffer >= buffers.len() {
        return Err(SolidError::parse(format!(
            "glTF: bufferView {bv_idx} references buffer {}, but only {} exist",
            bv.buffer,
            buffers.len()
        )));
    }
    let buf = &buffers[bv.buffer];
    let start = bv
        .byte_offset
        .checked_add(acc.byte_offset)
        .ok_or_else(|| SolidError::parse("glTF: accessor byte offset overflow"))?;
    let n_comps = num_components(&acc.type_);
    let comp_sz = component_size(acc.component_type);
    let stride = bv.byte_stride.unwrap_or(n_comps * comp_sz);
    // Bound the slice to the bufferView's declared byte_length (glTF 2.0
    // requires accessor data to be fully contained within its view).
    let view_end = bv
        .byte_offset
        .checked_add(bv.byte_length)
        .ok_or_else(|| SolidError::parse("glTF: bufferView byte_length overflow"))?;
    if start > buf.len() {
        return Err(SolidError::parse(format!(
            "glTF: accessor starts at byte {start} but buffer {} has {} bytes",
            bv.buffer,
            buf.len()
        )));
    }
    if start > view_end {
        return Err(SolidError::parse(format!(
            "glTF: accessor starts at byte {start} beyond bufferView {bv_idx} (ends at {view_end})"
        )));
    }
    let end = view_end.min(buf.len());
    let slice = &buf[start..end];
    Ok((slice, stride))
}

/// Validate an accessor index and its declared byte footprint before decoding.
fn validate_accessor(root: &GltfRoot, acc_idx: usize) -> Result<&GltfAccessor> {
    let acc = root
        .accessors
        .get(acc_idx)
        .ok_or_else(|| SolidError::parse(format!("glTF: accessor {acc_idx} out of range")))?;
    let n_comps = num_components(&acc.type_);
    let comp_sz = component_size(acc.component_type);
    let bytes = acc
        .count
        .checked_mul(n_comps)
        .and_then(|v| v.checked_mul(comp_sz))
        .ok_or_else(|| SolidError::parse("glTF: accessor byte length overflow"))?;
    if bytes > MAX_ACCESSOR_BYTES {
        return Err(SolidError::parse(format!(
            "glTF: accessor {acc_idx} requires {bytes} bytes, exceeding the safety limit"
        )));
    }
    Ok(acc)
}

/// Ensure the strided element range `count * stride` (plus one full element)
/// fits inside `slice`. Returns the offset of the last element's first byte.
fn check_stride_fits(acc_idx: usize, count: usize, stride: usize, comp_bytes: usize, slice_len: usize) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    let last = (count - 1)
        .checked_mul(stride)
        .ok_or_else(|| SolidError::parse("glTF: accessor stride overflow"))?;
    let need = last
        .checked_add(comp_bytes)
        .ok_or_else(|| SolidError::parse("glTF: accessor byte range overflow"))?;
    if need > slice_len {
        return Err(SolidError::parse(format!(
            "glTF: accessor {acc_idx} needs {need} bytes but its bufferView provides {slice_len}"
        )));
    }
    Ok(())
}

/// Decode a single f32 from a byte slice at `off` using the given component type.
fn decode_f32(slice: &[u8], off: usize, component_type: u32, normalized: bool) -> f32 {
    match component_type {
        5126 => f32::from_le_bytes(slice[off..off + 4].try_into().unwrap()),
        5123 if normalized => {
            let v = u16::from_le_bytes(slice[off..off + 2].try_into().unwrap());
            v as f32 / 65535.0
        }
        5121 if normalized => slice[off] as f32 / 255.0,
        5122 if normalized => {
            let v = i16::from_le_bytes(slice[off..off + 2].try_into().unwrap());
            v as f32 / 32767.0
        }
        5120 if normalized => slice[off] as i8 as f32 / 127.0,
        5125 if normalized => {
            let v = u32::from_le_bytes(slice[off..off + 4].try_into().unwrap());
            v as f32 / 4294967295.0
        }
        5121 => slice[off] as f32,
        5123 => u16::from_le_bytes(slice[off..off + 2].try_into().unwrap()) as f32,
        5120 => slice[off] as i8 as f32,
        5122 => i16::from_le_bytes(slice[off..off + 2].try_into().unwrap()) as f32,
        5125 => u32::from_le_bytes(slice[off..off + 4].try_into().unwrap()) as f32,
        _ => 0.0,
    }
}

/// Bounds-checked decode for sparse overrides: returns `None` when `off` is
/// out of range so malformed sparse data errors instead of panicking.
fn decode_f32_checked(
    slice: &[u8],
    off: usize,
    component_type: u32,
    normalized: bool,
) -> Result<f32> {
    let sz = component_size(component_type);
    let end = off
        .checked_add(sz)
        .ok_or_else(|| SolidError::parse("glTF: sparse byte offset overflow"))?;
    if end > slice.len() {
        return Err(SolidError::parse(format!(
            "glTF: sparse accessor read at byte {off} (size {sz}) beyond {} available",
            slice.len()
        )));
    }
    Ok(decode_f32(slice, off, component_type, normalized))
}

/// Resolve a validated byte slice for a sparse-accessor bufferView reference.
fn sparse_slice<'a>(
    root: &GltfRoot,
    buffers: &'a [Vec<u8>],
    label: &str,
    bv_i: usize,
    byte_off: usize,
) -> Result<&'a [u8]> {
    if bv_i >= root.buffer_views.len() {
        return Err(SolidError::parse(format!(
            "glTF: sparse {label} bufferView {bv_i} out of range ({} views)",
            root.buffer_views.len()
        )));
    }
    let bv = &root.buffer_views[bv_i];
    if bv.buffer >= buffers.len() {
        return Err(SolidError::parse(format!(
            "glTF: sparse {label} bufferView {bv_i} references buffer {} out of range ({} buffers)",
            bv.buffer,
            buffers.len()
        )));
    }
    let buf = &buffers[bv.buffer];
    let start = bv
        .byte_offset
        .checked_add(byte_off)
        .ok_or_else(|| SolidError::parse("glTF: sparse byte offset overflow"))?;
    let end = bv
        .byte_offset
        .checked_add(bv.byte_length)
        .unwrap_or(buf.len())
        .min(buf.len());
    if start > end {
        return Err(SolidError::parse(format!(
            "glTF: sparse {label} starts at byte {start} beyond available {end}"
        )));
    }
    Ok(&buf[start..end])
}

/// Apply a glTF sparse accessor override to an already-populated f32 buffer.
fn apply_sparse_f32(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    sparse: &serde_json::Value,
    component_type: u32,
    normalized: bool,
    out: &mut Vec<f32>,
    n_comps: usize,
) -> Result<()> {
    let sparse_count = sparse["count"].as_u64().unwrap_or(0) as usize;
    if sparse_count == 0 {
        return Ok(());
    }

    let idx_obj = &sparse["indices"];
    let idx_bv_i = idx_obj["bufferView"].as_u64().unwrap_or(0) as usize;
    let idx_off = idx_obj["byteOffset"].as_u64().unwrap_or(0) as usize;
    let idx_ctype = idx_obj["componentType"].as_u64().unwrap_or(5125) as u32;
    let idx_csz = component_size(idx_ctype);
    let idx_slice = sparse_slice(root, buffers, "indices", idx_bv_i, idx_off)?;

    let val_obj = &sparse["values"];
    let val_bv_i = val_obj["bufferView"].as_u64().unwrap_or(0) as usize;
    let val_off = val_obj["byteOffset"].as_u64().unwrap_or(0) as usize;
    let val_csz = component_size(component_type);
    let val_slice = sparse_slice(root, buffers, "values", val_bv_i, val_off)?;

    check_stride_fits(0, sparse_count, idx_csz, idx_csz, idx_slice.len())?;
    let val_total = sparse_count
        .checked_mul(n_comps)
        .and_then(|v| v.checked_mul(val_csz))
        .ok_or_else(|| SolidError::parse("glTF: sparse value range overflow"))?;
    if val_total > val_slice.len() {
        return Err(SolidError::parse(format!(
            "glTF: sparse values need {val_total} bytes but bufferView provides {}",
            val_slice.len()
        )));
    }

    for k in 0..sparse_count {
        let idx_pos = k * idx_csz;
        let tgt = match idx_ctype {
            5121 => idx_slice[idx_pos] as usize,
            5123 => {
                u16::from_le_bytes(idx_slice[idx_pos..idx_pos + 2].try_into().unwrap()) as usize
            }
            5125 => {
                u32::from_le_bytes(idx_slice[idx_pos..idx_pos + 4].try_into().unwrap()) as usize
            }
            _ => continue,
        };
        for c in 0..n_comps {
            let src = (k * n_comps + c) * val_csz;
            let val = decode_f32_checked(val_slice, src, component_type, normalized)?;
            let dst = tgt * n_comps + c;
            if dst < out.len() {
                out[dst] = val;
            }
        }
    }
    Ok(())
}

/// Apply a glTF sparse accessor override to an already-populated u32 buffer.
fn apply_sparse_u32(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    sparse: &serde_json::Value,
    component_type: u32,
    out: &mut Vec<u32>,
) -> Result<()> {
    let sparse_count = sparse["count"].as_u64().unwrap_or(0) as usize;
    if sparse_count == 0 {
        return Ok(());
    }

    let idx_obj = &sparse["indices"];
    let idx_bv_i = idx_obj["bufferView"].as_u64().unwrap_or(0) as usize;
    let idx_off = idx_obj["byteOffset"].as_u64().unwrap_or(0) as usize;
    let idx_ctype = idx_obj["componentType"].as_u64().unwrap_or(5125) as u32;
    let idx_csz = component_size(idx_ctype);
    let idx_slice = sparse_slice(root, buffers, "indices", idx_bv_i, idx_off)?;

    let val_obj = &sparse["values"];
    let val_bv_i = val_obj["bufferView"].as_u64().unwrap_or(0) as usize;
    let val_off = val_obj["byteOffset"].as_u64().unwrap_or(0) as usize;
    let val_csz = component_size(component_type);
    let val_slice = sparse_slice(root, buffers, "values", val_bv_i, val_off)?;

    check_stride_fits(0, sparse_count, idx_csz, idx_csz, idx_slice.len())?;
    let val_total = sparse_count
        .checked_mul(val_csz)
        .ok_or_else(|| SolidError::parse("glTF: sparse value range overflow"))?;
    if val_total > val_slice.len() {
        return Err(SolidError::parse(format!(
            "glTF: sparse values need {val_total} bytes but bufferView provides {}",
            val_slice.len()
        )));
    }

    for k in 0..sparse_count {
        let idx_pos = k * idx_csz;
        let tgt = match idx_ctype {
            5121 => idx_slice[idx_pos] as usize,
            5123 => {
                u16::from_le_bytes(idx_slice[idx_pos..idx_pos + 2].try_into().unwrap()) as usize
            }
            5125 => {
                u32::from_le_bytes(idx_slice[idx_pos..idx_pos + 4].try_into().unwrap()) as usize
            }
            _ => continue,
        };
        let src = k * val_csz;
        let val = match component_type {
            5121 => val_slice[src] as u32,
            5123 => u16::from_le_bytes(val_slice[src..src + 2].try_into().unwrap()) as u32,
            5125 => u32::from_le_bytes(val_slice[src..src + 4].try_into().unwrap()),
            _ => 0,
        };
        if tgt < out.len() {
            out[tgt] = val;
        }
    }
    Ok(())
}

/// Read an accessor as f32 values. Handles FLOAT, normalized integer types,
/// sparse overrides, and accessors with no bufferView (all-zero base for
/// sparse-only). Returns a flat vec of length
/// `accessor.count * num_components(accessor.type_)`.
pub fn read_f32(root: &GltfRoot, buffers: &[Vec<u8>], acc_idx: usize) -> Result<Vec<f32>> {
    let acc = validate_accessor(root, acc_idx)?;
    let n_comps = num_components(&acc.type_);
    let comp_sz = component_size(acc.component_type);

    let mut out = if acc.buffer_view.is_some() {
        let (slice, stride) = get_slice(root, buffers, acc)?;
        check_stride_fits(acc_idx, acc.count, stride, n_comps * comp_sz, slice.len())?;
        let mut out = Vec::with_capacity(acc.count * n_comps);
        for i in 0..acc.count {
            let base = i * stride;
            for c in 0..n_comps {
                out.push(decode_f32(
                    slice,
                    base + c * comp_sz,
                    acc.component_type,
                    acc.normalized,
                ));
            }
        }
        out
    } else {
        vec![0.0f32; acc.count * n_comps]
    };

    if let Some(sparse) = &acc.sparse {
        apply_sparse_f32(
            root,
            buffers,
            sparse,
            acc.component_type,
            acc.normalized,
            &mut out,
            n_comps,
        )?;
    }

    Ok(out)
}

/// Read an accessor as u32 indices. Handles UNSIGNED_BYTE/SHORT/INT,
/// sparse overrides, and accessors with no bufferView.
pub fn read_u32(root: &GltfRoot, buffers: &[Vec<u8>], acc_idx: usize) -> Result<Vec<u32>> {
    let acc = validate_accessor(root, acc_idx)?;

    let mut out = if acc.buffer_view.is_some() {
        let (slice, stride) = get_slice(root, buffers, acc)?;
        check_stride_fits(acc_idx, acc.count, stride, component_size(acc.component_type), slice.len())?;
        let mut out = Vec::with_capacity(acc.count);
        for i in 0..acc.count {
            let off = i * stride;
            let v = match acc.component_type {
                5121 => slice[off] as u32,
                5123 => u16::from_le_bytes(slice[off..off + 2].try_into().unwrap()) as u32,
                5125 => u32::from_le_bytes(slice[off..off + 4].try_into().unwrap()),
                _ => 0,
            };
            out.push(v);
        }
        out
    } else {
        vec![0u32; acc.count]
    };

    if let Some(sparse) = &acc.sparse {
        apply_sparse_u32(root, buffers, sparse, acc.component_type, &mut out)?;
    }

    Ok(out)
}

/// Read u16 values (used for JOINTS_0).
pub fn read_u16_vec4(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    acc_idx: usize,
) -> Result<Vec<[u16; 4]>> {
    let acc = validate_accessor(root, acc_idx)?;
    let (slice, stride) = get_slice(root, buffers, acc)?;
    let comp_sz = component_size(acc.component_type);
    check_stride_fits(acc_idx, acc.count, stride, 4 * comp_sz, slice.len())?;
    let mut out = Vec::with_capacity(acc.count);
    for i in 0..acc.count {
        let base = i * stride;
        let mut joints = [0u16; 4];
        for c in 0..4 {
            let off = base + c * comp_sz;
            joints[c] = match acc.component_type {
                5121 => slice[off] as u16,
                5123 => u16::from_le_bytes(slice[off..off + 2].try_into().unwrap()),
                _ => 0,
            };
        }
        out.push(joints);
    }
    Ok(out)
}
