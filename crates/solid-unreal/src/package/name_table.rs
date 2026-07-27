use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::types::FNameEntry;

/// Reads the name table from a UE package.
///
/// The name table is an array of `FNameEntry` serialized entries.
/// In UE4, each entry has a length-prefixed name and flags.
/// In UE5, the format changes to include precomputed hashes and
/// optional wide-character storage.
pub fn read_name_table(
    archive: &mut FArchiveUE,
    count: i32,
) -> Result<Vec<FNameEntry>, UnrealError> {
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        entries.push(read_name_entry(archive)?);
    }
    Ok(entries)
}

/// Reads name entries using the cooked inline format:
/// [length:i32][string:length bytes including null] repeated until length=0.
pub fn read_name_table_cooked(
    archive: &mut FArchiveUE,
) -> Result<Vec<FNameEntry>, UnrealError> {
    let mut entries = Vec::new();
    loop {
        let len = archive.read_i32()?;
        if len <= 0 {
            // Length 0 or negative terminates the table (or indicates wide string)
            if len < 0 {
                // Wide string
                let abs_len = (-len) as usize;
                let mut raw = vec![0u8; abs_len * 2];
                archive.read_exact(&mut raw)?;
                let u16_data: Vec<u16> = raw.chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                let trimmed: Vec<u16> = u16_data.into_iter().take_while(|&c| c != 0).collect();
                entries.push(FNameEntry {
                    text: String::from_utf16_lossy(&trimmed),
                    hash: None,
                    is_wide: true,
                });
            } else {
                break; // length 0 = terminator
            }
        } else {
            let mut raw = vec![0u8; len as usize];
            archive.read_exact(&mut raw)?;
            while raw.last() == Some(&0) { raw.pop(); }
            entries.push(FNameEntry {
                text: String::from_utf8_lossy(&raw).into_owned(),
                hash: None,
                is_wide: false,
            });
        }
    }
    Ok(entries)
}

/// Reads name entries until a zero-length entry is encountered.
/// Used for compact cooked format (ver=-5) where the count is not stored.
pub fn read_name_table_until_end(
    archive: &mut FArchiveUE,
) -> Result<Vec<FNameEntry>, UnrealError> {
    let mut entries = Vec::new();
    loop {
        match read_name_entry(archive) {
            Ok(entry) => {
                // Zero-length entries terminate the table
                if entry.text.is_empty() {
                    break;
                }
                entries.push(entry);
            }
            Err(_) => break,
        }
    }
    Ok(entries)
}

fn read_name_entry(archive: &mut FArchiveUE) -> Result<FNameEntry, UnrealError> {
    let ver = archive.version().clone();

    if ver.is_ue5() {
        // UE5: FNameEntry has hash/wide flags before the string
        read_name_entry_ue5(archive)
    } else if ver.is_cooked() && ver.is_ue4() {
        // Cooked UE4: uses u32 flags instead of u16
        read_name_entry_ue4_cooked(archive)
    } else if ver.is_cooked() {
        // Other cooked: no-flags format
        read_name_entry_cooked(archive)
    } else {
        // UE4 uncooked: standard format
        read_name_entry_ue4(archive)
    }
}

/// UE4-style FNameEntry:
///   serial_size(u32) | text(serial_size bytes, null-terminated+padding) | flags(u16)
fn read_name_entry_ue4(archive: &mut FArchiveUE) -> Result<FNameEntry, UnrealError> {
    read_name_entry_ue4_impl(archive, false)
}

/// Cooked UE4-style FNameEntry:
///   serial_size(u32) | text(serial_size bytes, null-terminated+padding) | flags(u32)
/// Cooked packages use u32 flags instead of u16.
fn read_name_entry_ue4_cooked(archive: &mut FArchiveUE) -> Result<FNameEntry, UnrealError> {
    read_name_entry_ue4_impl(archive, true)
}

fn read_name_entry_ue4_impl(archive: &mut FArchiveUE, cooked: bool) -> Result<FNameEntry, UnrealError> {
    // UE4 uses a 32-bit size that includes the null terminator
    let raw_size = archive.read_u32()? as i32;

    if raw_size < 0 {
        // Wide string
        let len = (-raw_size) as usize;
        let mut raw = vec![0u8; len * 2];
        archive.read_exact(&mut raw)?;
        let u16_data: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let trimmed: Vec<u16> = u16_data.into_iter().take_while(|&c| c != 0).collect();
        let text = String::from_utf16_lossy(&trimmed);
        if cooked { let _flags = archive.read_u32()?; }
        else { let _flags = archive.read_u16()?; }
        Ok(FNameEntry {
            text,
            hash: None,
            is_wide: true,
        })
    } else {
        // ANSI string including null terminator
        let len = raw_size as usize;
        let mut raw = vec![0u8; len];
        archive.read_exact(&mut raw)?;
        // Trim null terminator
        let trimmed: Vec<u8> = raw.into_iter().take_while(|&b| b != 0).collect();
        let text = String::from_utf8_lossy(&trimmed).into_owned();
        if cooked { let _flags = archive.read_u32()?; }
        else { let _flags = archive.read_u16()?; }
        Ok(FNameEntry {
            text,
            hash: None,
            is_wide: false,
        })
    }
}

/// UE5-style FNameEntry:
///   flags(u16) | [hash(u64)] | [wide_prefix(u32)] | text
fn read_name_entry_ue5(archive: &mut FArchiveUE) -> Result<FNameEntry, UnrealError> {
    let flags = archive.read_u16()?;

    // Whether this entry has a precomputed hash
    const FNAME_ENTRY_EXT_HASH: u16 = 0x8000;
    // Whether the string is wide (UTF-16)
    const FNAME_ENTRY_EXT_WIDE: u16 = 0x4000;

    let has_hash = (flags & FNAME_ENTRY_EXT_HASH) != 0;
    let is_wide = (flags & FNAME_ENTRY_EXT_WIDE) != 0;

    let hash = if has_hash {
        Some(archive.read_u64()?)
    } else {
        None
    };

    if is_wide {
        // Read length prefix (u32) then UCS-2 data (null-terminated)
        let len = archive.read_u32()? as usize;
        let mut raw = vec![0u8; len * 2];
        archive.read_exact(&mut raw)?;
        let u16_data: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let trimmed: Vec<u16> = u16_data.into_iter().take_while(|&c| c != 0).collect();
        let text = String::from_utf16_lossy(&trimmed);
        Ok(FNameEntry {
            text,
            hash,
            is_wide: true,
        })
    } else {
        // Read ANSI string (null-terminated, no length prefix)
        let mut text = Vec::new();
        loop {
            let byte = archive.read_u8()?;
            if byte == 0 {
                break;
            }
            text.push(byte);
        }
        Ok(FNameEntry {
            text: String::from_utf8_lossy(&text).into_owned(),
            hash,
            is_wide: false,
        })
    }
}

/// Cooked/UE5-style name entry: length(i32) + string(including null).
/// No flags, no hash, no wide prefix — just length-prefixed null-terminated strings.
fn read_name_entry_cooked(archive: &mut FArchiveUE) -> Result<FNameEntry, UnrealError> {
    let len = archive.read_i32()?;
    // Safety: reject absurd lengths (> 1MB for a single name)
    if len > 1_048_576 || len < -1_048_576 {
        return Err(UnrealError::Parse {
            context: "read_name_entry_cooked",
            detail: format!("suspicious name entry length {len}"),
        });
    }
    if len > 0 {
        // ANSI string including null terminator
        let mut raw = vec![0u8; len as usize];
        archive.read_exact(&mut raw)?;
        // Trim null terminator(s)
        while raw.last() == Some(&0) {
            raw.pop();
        }
        let text = String::from_utf8_lossy(&raw).into_owned();
        Ok(FNameEntry {
            text,
            hash: None,
            is_wide: false,
        })
    } else if len < 0 {
        // Wide string (negative length)
        let abs_len = (-len) as usize;
        let mut raw = vec![0u8; abs_len * 2];
        archive.read_exact(&mut raw)?;
        let u16_data: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let trimmed: Vec<u16> = u16_data.into_iter().take_while(|&c| c != 0).collect();
        let text = String::from_utf16_lossy(&trimmed);
        Ok(FNameEntry {
            text,
            hash: None,
            is_wide: true,
        })
    } else {
        // Empty name (len == 0)
        Ok(FNameEntry {
            text: String::new(),
            hash: None,
            is_wide: false,
        })
    }
}
