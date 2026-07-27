use std::io::SeekFrom;

use crate::error::UnrealError;
use crate::types::{CompressionMethod, FName, PackageIndex};
use crate::version::PackageVersion;

/// A version-aware serialization archive that wraps a `Read + Seek` stream.
///
/// The archive tracks the UE package version and provides typed read methods
/// that dispatch on version where the serialization format differs between
/// UE4 and UE5.
pub struct FArchiveUE<'a> {
    reader: &'a mut (dyn solid_rs::traits::ReadSeek),
    ver: PackageVersion,
    /// Position tracking for error reporting.
    pub pos: u64,
}

impl<'a> FArchiveUE<'a> {
    pub fn new(
        reader: &'a mut (dyn solid_rs::traits::ReadSeek),
        ver: PackageVersion,
    ) -> Self {
        Self {
            reader,
            ver,
            pos: 0,
        }
    }

    pub fn version(&self) -> &PackageVersion {
        &self.ver
    }

    /// Get a mutable reference to the version for updating after header parse.
    pub fn version_mut(&mut self) -> &mut PackageVersion {
        &mut self.ver
    }

    // ── Raw read helpers ──────────────────────────────────────────────────

    #[inline]
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), UnrealError> {
        self.reader.read_exact(buf)?;
        self.pos += buf.len() as u64;
        Ok(())
    }

    #[inline]
    pub fn seek(&mut self, _offset: i64, whence: SeekFrom) -> Result<u64, UnrealError> {
        let new_pos = self.reader.seek(whence)?;
        self.pos = new_pos;
        Ok(new_pos)
    }

    #[inline]
    pub fn seek_to(&mut self, offset: u64) -> Result<u64, UnrealError> {
        let new_pos = self.reader.seek(SeekFrom::Start(offset))?;
        self.pos = new_pos;
        Ok(new_pos)
    }

    #[inline]
    pub fn skip(&mut self, count: u64) -> Result<(), UnrealError> {
        let new_pos = self.reader.seek(SeekFrom::Current(count as i64))?;
        self.pos = new_pos;
        Ok(())
    }

    // ── Primitive reads ───────────────────────────────────────────────────

    pub fn read_u8(&mut self) -> Result<u8, UnrealError> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn read_i8(&mut self) -> Result<i8, UnrealError> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_u16(&mut self) -> Result<u16, UnrealError> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn read_i16(&mut self) -> Result<i16, UnrealError> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    pub fn read_u32(&mut self) -> Result<u32, UnrealError> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn read_i32(&mut self) -> Result<i32, UnrealError> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub fn read_u64(&mut self) -> Result<u64, UnrealError> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    pub fn read_i64(&mut self) -> Result<i64, UnrealError> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }

    pub fn read_f32(&mut self) -> Result<f32, UnrealError> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    pub fn read_f64(&mut self) -> Result<f64, UnrealError> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }

    pub fn read_bool(&mut self) -> Result<bool, UnrealError> {
        Ok(self.read_u32()? != 0)
    }

    /// Read a 32-bit serialized size as u32 (UE4) or i64 (UE5+).
    pub fn read_serial_size(&mut self) -> Result<i64, UnrealError> {
        if self.ver.is_ue5() {
            self.read_i64()
        } else {
            Ok(self.read_i32()? as i64)
        }
    }

    /// Read a serialized offset: u32 in UE4, i64 in UE5+.
    pub fn read_serial_offset(&mut self) -> Result<i64, UnrealError> {
        if self.ver.is_ue5() {
            self.read_i64()
        } else {
            Ok(self.read_i32()? as i64)
        }
    }

    // ── String reads ──────────────────────────────────────────────────────

    /// Read an FString (UE serialized string: 4/8-byte length prefix + UTF-16 or ANSI data).
    pub fn read_fstring(&mut self) -> Result<String, UnrealError> {
        let len = self.read_serial_size()?;

        if len == 0 {
            return Ok(String::new());
        }

        let abs_len = len.unsigned_abs() as usize;
        let is_wide = len < 0;

        if is_wide {
            // UCS-2 / UTF-16 encoded
            let mut raw = vec![0u8; abs_len * 2];
            self.read_exact(&mut raw)?;
            let u16_data: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            // Trim null terminator
            let trimmed: Vec<u16> = u16_data.into_iter().take_while(|&c| c != 0).collect();
            Ok(String::from_utf16_lossy(&trimmed))
        } else {
            // ANSI encoded
            let mut raw = vec![0u8; abs_len];
            self.read_exact(&mut raw)?;
            // Trim null terminator
            while raw.last() == Some(&0) {
                raw.pop();
            }
            Ok(String::from_utf8_lossy(&raw).into_owned())
        }
    }

    /// Read an FName (serialized as name index + number).
    pub fn read_fname(&mut self) -> Result<FName, UnrealError> {
        let index = self.read_i32()?;
        let number = self.read_i32()?;
        Ok(FName::new(index, number))
    }

    /// Read an FPackageIndex (serialized as i32).
    pub fn read_package_index(&mut self) -> Result<PackageIndex, UnrealError> {
        Ok(PackageIndex(self.read_i32()?))
    }

    /// Read a GUID (16 bytes).
    pub fn read_guid(&mut self) -> Result<[u8; 16], UnrealError> {
        let mut guid = [0u8; 16];
        self.read_exact(&mut guid)?;
        Ok(guid)
    }

    // ── TArray read ────────────────────────────────────────────────────────

    /// Read a length-prefixed array of items using the given read function.
    pub fn read_array<T>(
        &mut self,
        read_item: &mut impl FnMut(&mut Self) -> Result<T, UnrealError>,
    ) -> Result<Vec<T>, UnrealError> {
        let len = self.read_serial_size()? as usize;
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(read_item(self)?);
        }
        Ok(items)
    }

    // ── Compression ───────────────────────────────────────────────────────

    /// Read a compressed block header, returning the block descriptors.
    pub fn read_compression_info(&mut self) -> Result<super::types::CompressionInfo, UnrealError> {
        use crate::types::{CompressedBlock, CompressionInfo};

        let method_val = self.read_u32()?;
        let method = match method_val {
            0 => return Err(UnrealError::Parse {
                context: "compression_info",
                detail: "compression method is None (data not compressed)".into(),
            }),
            1 => CompressionMethod::Zlib,
            2 => CompressionMethod::LZ4,
            3 => CompressionMethod::Oodle,
            _ => return Err(UnrealError::Parse {
                context: "compression_info",
                detail: format!("unknown compression method {method_val}"),
            }),
        };

        let block_size = self.read_i32()?;
        let block_count = self.read_i32()?;

        let mut blocks = Vec::with_capacity(block_count as usize);
        for _ in 0..block_count {
            let uncompressed_offset = self.read_i64()?;
            let uncompressed_size = self.read_i64()?;
            let compressed_offset = self.read_i64()?;
            let compressed_size = self.read_i64()?;
            blocks.push(CompressedBlock {
                uncompressed_offset,
                uncompressed_size,
                compressed_size,
                compressed_offset,
            });
        }

        Ok(CompressionInfo {
            method,
            block_size,
            blocks,
        })
    }

    // ── Generic header extensions (UE5+) ──────────────────────────────────

    /// Read UE5+ header extension tags.
    pub fn read_header_extensions(
        &mut self,
        count: i32,
    ) -> Result<Vec<(FName, Vec<u8>)>, UnrealError> {
        let mut extensions = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let tag = self.read_fname()?;
            let size = self.read_serial_size()?;
            let mut data = vec![0u8; size as usize];
            self.read_exact(&mut data)?;
            extensions.push((tag, data));
        }
        Ok(extensions)
    }
}
