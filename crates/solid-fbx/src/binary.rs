//! Binary FBX parser.
//!
//! Supports both the "classic" 32-bit offset format (FBX versions < 7500) and
//! the extended 64-bit offset format (FBX 7.5 / MotionBuilder 2016+).
//! Zlib-compressed array properties are decompressed transparently using
//! `flate2`.

use std::io::{Read, Seek, SeekFrom};

use flate2::read::ZlibDecoder;
use solid_rs::traits::ReadSeek;
use solid_rs::{Result, SolidError};

use crate::document::{FbxDocument, FbxNode, FbxProperty};

// ── Magic ─────────────────────────────────────────────────────────────────────

const MAGIC: &[u8; 23] = b"Kaydara FBX Binary  \x00\x1a\x00";

/// Peek at the first 23 bytes and check for the binary FBX magic.
/// The reader position is restored afterwards.
pub(crate) fn detect(reader: &mut dyn ReadSeek) -> bool {
    let mut buf = [0u8; 23];
    let ok = reader.read_exact(&mut buf).is_ok() && &buf == MAGIC;
    let _ = reader.seek(SeekFrom::Start(0));
    ok
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Parse a binary FBX document from `reader`.
pub(crate) fn parse(reader: &mut dyn ReadSeek) -> Result<FbxDocument> {
    let mut magic = [0u8; 23];
    reader.read_exact(&mut magic).map_err(SolidError::Io)?;
    if &magic != MAGIC {
        return Err(SolidError::parse("not a binary FBX file (bad magic)"));
    }

    let mut p = BinaryParser::new(reader)?;
    let version = p.version;
    let mut roots = Vec::new();
    loop {
        match p.read_node()? {
            Some(node) => roots.push(node),
            None => break,
        }
    }
    Ok(FbxDocument { version, roots })
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct BinaryParser<'r> {
    r: &'r mut dyn ReadSeek,
    version: u32,
}

impl<'r> BinaryParser<'r> {
    fn new(r: &'r mut dyn ReadSeek) -> Result<Self> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf).map_err(SolidError::Io)?;
        let version = u32::from_le_bytes(buf);
        Ok(Self { r, version })
    }

    // ── Positional helpers ────────────────────────────────────────────────────

    fn pos(&mut self) -> Result<u64> {
        self.r.seek(SeekFrom::Current(0)).map_err(SolidError::Io)
    }

    fn seek_to(&mut self, offset: u64) -> Result<()> {
        self.r.seek(SeekFrom::Start(offset)).map(|_| ()).map_err(SolidError::Io)
    }

    // ── Primitive readers ─────────────────────────────────────────────────────

    fn read_u8(&mut self) -> Result<u8> {
        let mut b = [0u8; 1];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(b[0])
    }

    fn read_u32(&mut self) -> Result<u32> {
        let mut b = [0u8; 4];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(u32::from_le_bytes(b))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let mut b = [0u8; 8];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(u64::from_le_bytes(b))
    }

    fn read_i16(&mut self) -> Result<i16> {
        let mut b = [0u8; 2];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(i16::from_le_bytes(b))
    }

    fn read_i32(&mut self) -> Result<i32> {
        let mut b = [0u8; 4];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(i32::from_le_bytes(b))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let mut b = [0u8; 8];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(i64::from_le_bytes(b))
    }

    fn read_f32(&mut self) -> Result<f32> {
        let mut b = [0u8; 4];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(f32::from_le_bytes(b))
    }

    fn read_f64(&mut self) -> Result<f64> {
        let mut b = [0u8; 8];
        self.r.read_exact(&mut b).map_err(SolidError::Io)?;
        Ok(f64::from_le_bytes(b))
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.r.read_exact(&mut buf).map_err(SolidError::Io)?;
        Ok(buf)
    }

    // ── Node reading ─────────────────────────────────────────────────────────

    /// Size of the null-sentinel record that marks the end of a children list.
    fn null_record_len(&self) -> u64 {
        if self.version >= 7500 { 25 } else { 13 }
    }

    /// Read one node.  Returns `None` when a null sentinel is encountered
    /// (which signals the end of a children list or the document).
    fn read_node(&mut self) -> Result<Option<FbxNode>> {
        // Read the node header — size depends on format version
        let (end_offset, num_props): (u64, usize) = if self.version >= 7500 {
            let end = self.read_u64()?;
            let np  = self.read_u64()?;
            let _pl = self.read_u64()?; // properties byte-length (unused)
            (end, np as usize)
        } else {
            let end = self.read_u32()? as u64;
            let np  = self.read_u32()? as u64;
            let _pl = self.read_u32()?; // properties byte-length (unused)
            (end, np as usize)
        };

        let name_len = self.read_u8()? as usize;

        // All-zero header = null sentinel
        if end_offset == 0 && num_props == 0 && name_len == 0 {
            return Ok(None);
        }

        let name_bytes = self.read_bytes(name_len)?;
        let name = String::from_utf8_lossy(&name_bytes).into_owned();

        // Read properties
        let mut properties = Vec::with_capacity(num_props);
        for _ in 0..num_props {
            properties.push(self.read_property()?);
        }

        // Read nested children if there is room before end_offset
        let null_len = self.null_record_len();
        let mut children = Vec::new();
        loop {
            let pos = self.pos()?;
            if pos >= end_offset.saturating_sub(null_len) {
                break;
            }
            match self.read_node()? {
                Some(child) => children.push(child),
                None        => break,
            }
        }

        // Seek past any trailing data / null-sentinel we may not have consumed
        if end_offset > 0 {
            self.seek_to(end_offset)?;
        }

        Ok(Some(FbxNode { name, properties, children }))
    }

    // ── Property reading ──────────────────────────────────────────────────────

    fn read_property(&mut self) -> Result<FbxProperty> {
        let code = self.read_u8()?;
        match code {
            b'Y' => Ok(FbxProperty::Int16(self.read_i16()?)),
            b'C' => Ok(FbxProperty::Bool(self.read_u8()? != 0)),
            b'I' => Ok(FbxProperty::Int32(self.read_i32()?)),
            b'F' => Ok(FbxProperty::Float32(self.read_f32()?)),
            b'D' => Ok(FbxProperty::Float64(self.read_f64()?)),
            b'L' => Ok(FbxProperty::Int64(self.read_i64()?)),

            b'S' => {
                let len  = self.read_u32()? as usize;
                let data = self.read_bytes(len)?;
                Ok(FbxProperty::String(String::from_utf8_lossy(&data).into_owned()))
            }
            b'R' => {
                let len  = self.read_u32()? as usize;
                let data = self.read_bytes(len)?;
                Ok(FbxProperty::Bytes(data))
            }

            b'b' => {
                let raw = self.read_array_data(1)?;
                Ok(FbxProperty::ArrBool(raw.iter().map(|&b| b != 0).collect()))
            }
            b'i' => {
                let raw = self.read_array_data(4)?;
                Ok(FbxProperty::ArrInt32(
                    raw.chunks_exact(4)
                       .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
                       .collect(),
                ))
            }
            b'l' => {
                let raw = self.read_array_data(8)?;
                Ok(FbxProperty::ArrInt64(
                    raw.chunks_exact(8)
                       .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
                       .collect(),
                ))
            }
            b'f' => {
                let raw = self.read_array_data(4)?;
                Ok(FbxProperty::ArrFloat32(
                    raw.chunks_exact(4)
                       .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                       .collect(),
                ))
            }
            b'd' => {
                let raw = self.read_array_data(8)?;
                Ok(FbxProperty::ArrFloat64(
                    raw.chunks_exact(8)
                       .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
                       .collect(),
                ))
            }

            other => Err(SolidError::parse(format!(
                "unknown FBX property type code: 0x{other:02X} ('{}')",
                other as char
            ))),
        }
    }

    /// Read the array header (count / encoding / compressed-length) then
    /// return the raw decompressed bytes.
    fn read_array_data(&mut self, elem_size: usize) -> Result<Vec<u8>> {
        let count          = self.read_u32()? as usize;
        let encoding       = self.read_u32()?;
        let compressed_len = self.read_u32()? as usize;

        let raw = self.read_bytes(compressed_len)?;

        match encoding {
            0 => Ok(raw), // uncompressed
            1 => {
                // zlib-deflate
                let mut dec = ZlibDecoder::new(&raw[..]);
                let mut out = Vec::with_capacity(count * elem_size);
                dec.read_to_end(&mut out).map_err(SolidError::Io)?;
                Ok(out)
            }
            e => Err(SolidError::parse(format!("unknown FBX array encoding: {e}"))),
        }
    }
}
