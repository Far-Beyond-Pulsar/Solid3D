use std::io::{Read, Seek, SeekFrom};

use crate::error::UnrealError;

pub fn read_u8<R: Read>(r: &mut R) -> Result<u8, UnrealError> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}

pub fn read_u16<R: Read>(r: &mut R) -> Result<u16, UnrealError> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}

pub fn read_i16<R: Read>(r: &mut R) -> Result<i16, UnrealError> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(i16::from_le_bytes(b))
}

pub fn read_u32<R: Read>(r: &mut R) -> Result<u32, UnrealError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

pub fn read_i32<R: Read>(r: &mut R) -> Result<i32, UnrealError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}

pub fn read_u64<R: Read>(r: &mut R) -> Result<u64, UnrealError> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

pub fn read_i64<R: Read>(r: &mut R) -> Result<i64, UnrealError> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

pub fn read_f32<R: Read>(r: &mut R) -> Result<f32, UnrealError> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}

pub fn read_f64<R: Read>(r: &mut R) -> Result<f64, UnrealError> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(f64::from_le_bytes(b))
}

pub fn read_bool_u32<R: Read>(r: &mut R) -> Result<bool, UnrealError> {
    Ok(read_u32(r)? != 0)
}

pub fn read_guid<R: Read>(r: &mut R) -> Result<[u8; 16], UnrealError> {
    let mut guid = [0u8; 16];
    r.read_exact(&mut guid)?;
    Ok(guid)
}

pub fn read_fstring<R: Read>(r: &mut R) -> Result<String, UnrealError> {
    let len = read_i32(r)?;
    if len == 0 {
        return Ok(String::new());
    }
    let abs_len = len.unsigned_abs() as usize;
    if len < 0 {
        let mut raw = vec![0u8; abs_len * 2];
        r.read_exact(&mut raw)?;
        let u16_data: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let trimmed: Vec<u16> = u16_data.into_iter().take_while(|&c| c != 0).collect();
        Ok(String::from_utf16_lossy(&trimmed))
    } else {
        let mut raw = vec![0u8; abs_len];
        r.read_exact(&mut raw)?;
        while raw.last() == Some(&0) { raw.pop(); }
        Ok(String::from_utf8_lossy(&raw).into_owned())
    }
}

pub fn read_fname<R: Read>(r: &mut R) -> Result<uasset::NameReference, UnrealError> {
    let index = read_i32(r)?;
    let number: i32 = read_i32(r)?;
    Ok(uasset::NameReference {
        index: index as u32,
        number: std::num::NonZeroU32::new(number as u32),
    })
}

pub fn read_package_index<R: Read>(r: &mut R) -> Result<i32, UnrealError> {
    read_i32(r)
}

pub fn seek_to<R: Seek>(r: &mut R, offset: u64) -> Result<(), UnrealError> {
    r.seek(SeekFrom::Start(offset))?;
    Ok(())
}

pub fn resolve_name(names: &[String], nr: &uasset::NameReference) -> String {
    let idx = nr.index as usize;
    if idx < names.len() {
        let mut s = names[idx].clone();
        if let Some(num) = nr.number {
            s.push_str(&format!("_{}", num.get() - 1));
        }
        s
    } else {
        format!("<invalid_name_index {}>", nr.index)
    }
}
