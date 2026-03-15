use glam::{Vec3, Vec4};
use solid_rs::prelude::SolidError;

pub type Result<T> = std::result::Result<T, SolidError>;

#[derive(Debug, Clone)]
pub struct StlTriangle {
    pub normal: Vec3,
    pub vertices: [Vec3; 3],
    /// VisCAM/SolidView per-triangle color decoded from attribute bytes (bit 15 set).
    pub color: Option<Vec4>,
}

/// Returns true if `data` looks like a binary STL file.
/// Reliable detection: check if 80 + 4 + count*50 == file_len.
pub fn detect_binary(data: &[u8]) -> bool {
    if data.len() < 84 {
        return false;
    }
    let count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]) as usize;
    let expected = 84 + count * 50;
    expected == data.len()
}

pub fn parse_binary(data: &[u8]) -> Result<(String, Vec<StlTriangle>)> {
    if data.len() < 84 {
        return Err(SolidError::parse("STL binary: file too small"));
    }
    let count = u32::from_le_bytes([data[80], data[81], data[82], data[83]]) as usize;
    let expected = 84 + count * 50;
    if data.len() < expected {
        return Err(SolidError::parse("STL binary: file truncated"));
    }

    let name = std::str::from_utf8(&data[..80])
        .unwrap_or("")
        .trim_end_matches('\0')
        .trim()
        .to_string();

    let mut triangles = Vec::with_capacity(count);
    for i in 0..count {
        let base = 84 + i * 50;
        let normal = read_vec3(data, base);
        let v0 = read_vec3(data, base + 12);
        let v1 = read_vec3(data, base + 24);
        let v2 = read_vec3(data, base + 36);
        let off = base + 48;
        let attr = u16::from_le_bytes([data[off], data[off + 1]]);
        let color = if attr & 0x8000 != 0 {
            // VisCAM RGB555: bits 14-10=R, 9-5=G, 4-0=B
            let r = ((attr >> 10) & 0x1F) as f32 / 31.0;
            let g = ((attr >>  5) & 0x1F) as f32 / 31.0;
            let b = ( attr        & 0x1F) as f32 / 31.0;
            Some(Vec4::new(r, g, b, 1.0))
        } else {
            None
        };
        triangles.push(StlTriangle { normal, vertices: [v0, v1, v2], color });
    }
    Ok((name, triangles))
}

fn read_vec3(data: &[u8], offset: usize) -> Vec3 {
    let x = f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
    let y = f32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
    let z = f32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
    Vec3::new(x, y, z)
}

pub fn parse_ascii(data: &[u8]) -> Result<(String, Vec<StlTriangle>)> {
    let text = std::str::from_utf8(data).map_err(|_| SolidError::parse("STL: invalid UTF-8"))?;
    let mut name = String::new();
    let mut triangles = Vec::new();
    let mut current_normal = Vec3::ZERO;
    let mut verts: Vec<Vec3> = Vec::new();

    for line in text.lines().map(|l| l.trim()) {
        if line.starts_with("solid") {
            name = line[5..].trim().to_string();
        } else if line.starts_with("facet normal") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                current_normal = Vec3::new(
                    parts[2].parse().unwrap_or(0.0),
                    parts[3].parse().unwrap_or(0.0),
                    parts[4].parse().unwrap_or(0.0),
                );
            }
        } else if line.starts_with("vertex") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                verts.push(Vec3::new(
                    parts[1].parse().unwrap_or(0.0),
                    parts[2].parse().unwrap_or(0.0),
                    parts[3].parse().unwrap_or(0.0),
                ));
            }
        } else if line.starts_with("endfacet") {
            if verts.len() >= 3 {
                triangles.push(StlTriangle {
                    normal: current_normal,
                    vertices: [verts[0], verts[1], verts[2]],
                    color: None,
                });
            }
            verts.clear();
        }
    }
    Ok((name, triangles))
}
