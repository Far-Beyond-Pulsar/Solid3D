//! Binary parser for the Quake MDL model format.
//!
//! MDL is a little-endian binary format storing vertex positions as
//! unsigned bytes, decompressed via `v_real = scale * v_byte + translate`.
//! Normals are stored as 8-bit indices into a fixed table of 162
//! precomputed vectors (see [`constants::ANORMS`]).

use solid_rs::{Result, SolidError};

use crate::constants;

pub const MDL_IDENT: u32 = 1330660425;
pub const MDL_VERSION: i32 = 6;

pub const HEADER_SIZE: usize = 84;
pub const TEXCOORD_SIZE: usize = 12;
pub const TRIANGLE_SIZE: usize = 16;
pub const VERTEX_SIZE: usize = 4;
pub const SIMPLE_FRAME_FIXED_SIZE: usize = 24;

#[derive(Debug, Clone)]
pub struct MdlHeader {
    pub ident: u32,
    pub version: i32,
    pub scale: [f32; 3],
    pub translate: [f32; 3],
    pub boundingradius: f32,
    pub eyeposition: [f32; 3],
    pub num_skins: i32,
    pub skinwidth: i32,
    pub skinheight: i32,
    pub num_verts: i32,
    pub num_tris: i32,
    pub num_frames: i32,
    pub synctype: i32,
    pub flags: i32,
    pub size: f32,
}

#[derive(Debug, Clone)]
pub enum MdlSkin {
    Single { data: Vec<u8> },
    Group { nb: i32, times: Vec<f32>, data: Vec<Vec<u8>> },
}

#[derive(Debug, Clone)]
pub struct MdlTexCoord {
    pub onseam: i32,
    pub s: i32,
    pub t: i32,
}

#[derive(Debug, Clone)]
pub struct MdlTriangle {
    pub facesfront: i32,
    pub vertex: [i32; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct MdlVertex {
    pub v: [u8; 3],
    pub normal_index: u8,
}

#[derive(Debug, Clone)]
pub struct MdlSimpleFrame {
    pub bboxmin: MdlVertex,
    pub bboxmax: MdlVertex,
    pub name: [u8; 16],
    pub verts: Vec<MdlVertex>,
}

#[derive(Debug, Clone)]
pub enum MdlFrame {
    Simple(MdlSimpleFrame),
    Group {
        nb: i32,
        min: MdlVertex,
        max: MdlVertex,
        times: Vec<f32>,
        frames: Vec<MdlSimpleFrame>,
    },
}

#[derive(Debug, Clone)]
pub struct MdlData {
    pub header: MdlHeader,
    pub skins: Vec<MdlSkin>,
    pub texcoords: Vec<MdlTexCoord>,
    pub triangles: Vec<MdlTriangle>,
    pub frames: Vec<MdlFrame>,
}

// ── Binary reader helpers ──────────────────────────────────────────────────────

struct BinReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn ensure(&self, n: usize) -> Result<()> {
        if self.pos + n > self.data.len() {
            Err(SolidError::parse(format!(
                "unexpected end of MDL data at offset {} (needed {}, have {})",
                self.pos,
                n,
                self.data.len() - self.pos
            )))
        } else {
            Ok(())
        }
    }

    fn read_u32_le(&mut self) -> Result<u32> {
        self.ensure(4)?;
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_i32_le(&mut self) -> Result<i32> {
        self.ensure(4)?;
        let v = i32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_f32_le(&mut self) -> Result<f32> {
        self.ensure(4)?;
        let v = f32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_f32_3(&mut self) -> Result<[f32; 3]> {
        Ok([self.read_f32_le()?, self.read_f32_le()?, self.read_f32_le()?])
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        self.ensure(n)?;
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_vertex(&mut self) -> Result<MdlVertex> {
        let bytes = self.read_bytes(4)?;
        Ok(MdlVertex {
            v: [bytes[0], bytes[1], bytes[2]],
            normal_index: bytes[3],
        })
    }
}

// ── Header ─────────────────────────────────────────────────────────────────────

fn read_header(r: &mut BinReader) -> Result<MdlHeader> {
    let ident = r.read_u32_le()?;
    let version = r.read_i32_le()?;
    let scale = r.read_f32_3()?;
    let translate = r.read_f32_3()?;
    let boundingradius = r.read_f32_le()?;
    let eyeposition = r.read_f32_3()?;
    let num_skins = r.read_i32_le()?;
    let skinwidth = r.read_i32_le()?;
    let skinheight = r.read_i32_le()?;
    let num_verts = r.read_i32_le()?;
    let num_tris = r.read_i32_le()?;
    let num_frames = r.read_i32_le()?;
    let synctype = r.read_i32_le()?;
    let flags = r.read_i32_le()?;
    let size = r.read_f32_le()?;

    Ok(MdlHeader {
        ident,
        version,
        scale,
        translate,
        boundingradius,
        eyeposition,
        num_skins,
        skinwidth,
        skinheight,
        num_verts,
        num_tris,
        num_frames,
        synctype,
        flags,
        size,
    })
}

// ── Skins ──────────────────────────────────────────────────────────────────────

fn read_skins(r: &mut BinReader, header: &MdlHeader) -> Result<Vec<MdlSkin>> {
    let count = header.num_skins.max(0) as usize;
    let mut skins = Vec::with_capacity(count);
    for _ in 0..count {
        let group = r.read_i32_le()?;
        if group == 0 {
            let pixel_count = (header.skinwidth * header.skinheight).max(0) as usize;
            let data = r.read_bytes(pixel_count)?.to_vec();
            skins.push(MdlSkin::Single { data });
        } else {
            let nb = r.read_i32_le()?;
            let n = nb.max(0) as usize;
            let mut times = Vec::with_capacity(n);
            for _ in 0..n {
                times.push(r.read_f32_le()?);
            }
            let pixel_count = (header.skinwidth * header.skinheight).max(0) as usize;
            let mut data = Vec::with_capacity(n);
            for _ in 0..n {
                data.push(r.read_bytes(pixel_count)?.to_vec());
            }
            skins.push(MdlSkin::Group { nb, times, data });
        }
    }
    Ok(skins)
}

// ── Texture coordinates ────────────────────────────────────────────────────────

fn read_texcoords(r: &mut BinReader, count: usize) -> Result<Vec<MdlTexCoord>> {
    let mut texcoords = Vec::with_capacity(count);
    for _ in 0..count {
        let onseam = r.read_i32_le()?;
        let s = r.read_i32_le()?;
        let t = r.read_i32_le()?;
        texcoords.push(MdlTexCoord { onseam, s, t });
    }
    Ok(texcoords)
}

// ── Triangles ──────────────────────────────────────────────────────────────────

fn read_triangles(r: &mut BinReader, count: usize) -> Result<Vec<MdlTriangle>> {
    let mut triangles = Vec::with_capacity(count);
    for _ in 0..count {
        let facesfront = r.read_i32_le()?;
        let v0 = r.read_i32_le()?;
        let v1 = r.read_i32_le()?;
        let v2 = r.read_i32_le()?;
        triangles.push(MdlTriangle {
            facesfront,
            vertex: [v0, v1, v2],
        });
    }
    Ok(triangles)
}

// ── Frames ─────────────────────────────────────────────────────────────────────

fn read_simple_frame(r: &mut BinReader, num_verts: usize, _name_buf: &mut [u8; 16]) -> Result<MdlSimpleFrame> {
    let bboxmin = r.read_vertex()?;
    let bboxmax = r.read_vertex()?;
    let name_bytes = r.read_bytes(16)?;
    let name: [u8; 16] = {
        let mut buf = [0u8; 16];
        buf.copy_from_slice(name_bytes);
        buf
    };
    *_name_buf = name;
    let mut verts = Vec::with_capacity(num_verts);
    for _ in 0..num_verts {
        verts.push(r.read_vertex()?);
    }
    Ok(MdlSimpleFrame {
        bboxmin,
        bboxmax,
        name,
        verts,
    })
}

fn read_frames(r: &mut BinReader, header: &MdlHeader) -> Result<Vec<MdlFrame>> {
    let count = header.num_frames.max(0) as usize;
    let num_verts = header.num_verts.max(0) as usize;
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        let frame_type = r.read_i32_le()?;
        if frame_type == 0 {
            let mut name_buf = [0u8; 16];
            let sf = read_simple_frame(r, num_verts, &mut name_buf)?;
            frames.push(MdlFrame::Simple(sf));
        } else {
            let nb = r.read_i32_le()?;
            let n = nb.max(0) as usize;
            let min = r.read_vertex()?;
            let max = r.read_vertex()?;
            let mut times = Vec::with_capacity(n);
            for _ in 0..n {
                times.push(r.read_f32_le()?);
            }
            let mut sfs = Vec::with_capacity(n);
            for _ in 0..n {
                let mut name_buf = [0u8; 16];
                sfs.push(read_simple_frame(r, num_verts, &mut name_buf)?);
            }
            frames.push(MdlFrame::Group {
                nb,
                min,
                max,
                times,
                frames: sfs,
            });
        }
    }
    Ok(frames)
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Parse raw MDL bytes into a [`MdlData`] document.
pub fn parse_mdl(data: &[u8]) -> Result<MdlData> {
    if data.len() < HEADER_SIZE {
        return Err(SolidError::parse(format!(
            "MDL file too small: expected at least {} bytes, got {}",
            HEADER_SIZE,
            data.len()
        )));
    }

    let mut r = BinReader::new(data);

    let header = read_header(&mut r)?;

    if header.ident != MDL_IDENT {
        return Err(SolidError::parse(format!(
            "bad MDL identifier: expected {MDL_IDENT} (IDPO), got {}",
            header.ident
        )));
    }
    if header.version != MDL_VERSION {
        return Err(SolidError::parse(format!(
            "bad MDL version: expected {MDL_VERSION}, got {}",
            header.version
        )));
    }

    let num_verts = header.num_verts.max(0) as usize;
    let num_tris = header.num_tris.max(0) as usize;

    let skins = read_skins(&mut r, &header)?;
    let texcoords = read_texcoords(&mut r, num_verts)?;
    let triangles = read_triangles(&mut r, num_tris)?;
    let frames = read_frames(&mut r, &header)?;

    Ok(MdlData {
        header,
        skins,
        texcoords,
        triangles,
        frames,
    })
}

/// Decompress a byte vertex into a full float position.
pub fn decompress_vertex(v: &MdlVertex, scale: &[f32; 3], translate: &[f32; 3]) -> [f32; 3] {
    [
        scale[0] * v.v[0] as f32 + translate[0],
        scale[1] * v.v[1] as f32 + translate[1],
        scale[2] * v.v[2] as f32 + translate[2],
    ]
}

/// Compress a float position into a byte vertex given scale/translate.
pub fn compress_vertex(pos: [f32; 3], scale: &[f32; 3], translate: &[f32; 3]) -> MdlVertex {
    let v = [
        ((pos[0] - translate[0]) / scale[0]).round().clamp(0.0, 255.0) as u8,
        ((pos[1] - translate[1]) / scale[1]).round().clamp(0.0, 255.0) as u8,
        ((pos[2] - translate[2]) / scale[2]).round().clamp(0.0, 255.0) as u8,
    ];
    MdlVertex {
        v,
        normal_index: 0,
    }
}

/// Look up a normal vector from the anorms table.
pub fn get_normal(index: u8) -> glam::Vec3 {
    let idx = (index as usize).min(constants::ANORMS_COUNT - 1);
    constants::ANORMS[idx]
}
