use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::types::{ObjectExport, ObjectImport, PackageIndex};

/// Read the export table from a UE package.
/// Matches CUE4Parse FObjectExport serialization exactly.
pub fn read_export_table(
    archive: &mut FArchiveUE,
    count: i32,
) -> Result<Vec<ObjectExport>, UnrealError> {
    let is_cooked = archive.version().is_cooked();
    let ue4 = archive.version().ue4_raw;

    // For cooked UE4 packages, the export table uses a COMPACT format.
    // The compact format drops many fields present in uncooked:
    // - No SuperIndex
    // - No TemplateIndex  
    // - No Archetype
    // - SerialSize/SerialOffset are always i32
    // - No ForcedExport/NotForClient/NotForServer flags
    // The entry layout is:
    //   ClassIndex(i32) + OuterIndex(i32) + ObjectName(8) + ObjectFlags(u32)
    //   + SerialSize(i32) + SerialOffset(i32) + [PackageGuid(16)] + [extra flags]
    if is_cooked && ue4 >= 300 {
        return read_export_table_cooked(archive, count);
    }

    let mut exports = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let class_index = PackageIndex(archive.read_i32()?);
        let _super_index = PackageIndex(archive.read_i32()?);
        let _template_index = if ue4 >= 372 {
            PackageIndex(archive.read_i32()?)
        } else {
            PackageIndex(0)
        };
        let outer_index = PackageIndex(archive.read_i32()?);
        let object_name = archive.read_fname()?;

        if ue4 >= 278 && ue4 < 349 {
            let _archetype = PackageIndex(archive.read_i32()?);
        }

        let object_flags = archive.read_u32()?;

        let (serial_size, serial_offset) = if ue4 >= 348 {
            (archive.read_i64()?, archive.read_i64()?)
        } else {
            (archive.read_i32()? as i64, archive.read_i32()? as i64)
        };

        let _forced_export = if ue4 >= 258 { archive.read_u32()? != 0 } else { false };
        let _not_for_client = if ue4 >= 258 { archive.read_u32()? != 0 } else { false };
        let _not_for_server = if ue4 >= 258 { archive.read_u32()? != 0 } else { false };

        let _package_guid = if ue4 < 1003 {
            archive.read_guid()?
        } else {
            [0u8; 16]
        };

        let _is_inherited = if ue4 >= 1002 { archive.read_u32()? != 0 } else { false };

        if ue4 >= 267 && ue4 < 285 {
            let map_len = archive.read_serial_size()? as usize;
            for _ in 0..map_len {
                let _key = archive.read_fname()?;
                let _val = PackageIndex(archive.read_i32()?);
            }
        }

        if ue4 > 0 && ue4 < 100 {
            let _export_flags = archive.read_i32()?;
        }

        if ue4 >= 282 {
            if ue4 < 350 {
                let _net_count_len = archive.read_serial_size()?;
            }
            if ue4 >= 291 {
                let _pkg_flags = archive.read_u32()?;
            }
        }

        if !is_cooked && ue4 >= 377 {
            let _not_always_loaded = archive.read_u32()? != 0;
        }

        if !is_cooked && ue4 >= 383 {
            let _is_asset = archive.read_u32()? != 0;
        }

        let _gen_hash = if ue4 >= 1000 { archive.read_u32()? != 0 } else { false };

        if !is_cooked && ue4 >= 388 {
            let _first_dep = archive.read_i32()?;
            let _ser_before_ser = archive.read_i32()?;
            let _create_before_ser = archive.read_i32()?;
            let _ser_before_create = archive.read_i32()?;
            let _create_before_create = archive.read_i32()?;
        }

        let _script_start = if ue4 >= 1011 { archive.read_i64()? } else { 0 };
        let _script_end = if ue4 >= 1011 { archive.read_i64()? } else { 0 };

        exports.push(ObjectExport {
            class_index,
            outer_index,
            object_name,
            object_flags,
            serial_size,
            serial_offset,
            package_flags: 0,
            export_flags: 0,
            header_extensions: Vec::new(),
        });
    }
    Ok(exports)
}

/// Read export table for cooked UE4 packages (compact format).
fn read_export_table_cooked(
    archive: &mut FArchiveUE,
    count: i32,
) -> Result<Vec<ObjectExport>, UnrealError> {
    let ue4 = archive.version().ue4_raw;
    let has_guid = ue4 < 1003;

    let mut exports = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let class_index = PackageIndex(archive.read_i32()?);
        let _super_index = PackageIndex(archive.read_i32()?);
        let _template_index = PackageIndex(archive.read_i32()?);
        let outer_index = PackageIndex(archive.read_i32()?);
        let object_name = archive.read_fname()?;
        let object_flags = archive.read_u32()?;
        let serial_size = archive.read_i32()? as i64;
        let _pad = archive.read_u32()?;
        let serial_offset = archive.read_i32()? as i64;
        let _package_guid = if has_guid {
            archive.read_guid()?
        } else {
            [0u8; 16]
        };

        exports.push(ObjectExport {
            class_index,
            outer_index,
            object_name,
            object_flags,
            serial_size,
            serial_offset,
            package_flags: 0,
            export_flags: 0,
            header_extensions: Vec::new(),
        });
    }
    Ok(exports)
}

/// Read the import table from a UE package.
pub fn read_import_table(
    archive: &mut FArchiveUE,
    count: i32,
) -> Result<Vec<ObjectImport>, UnrealError> {
    let ver = archive.version();
    let ue4 = ver.ue4_raw;

    let mut imports = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let class_package = archive.read_fname()?;
        let class_name = archive.read_fname()?;
        let outer_index = PackageIndex(archive.read_i32()?);
        let object_name = archive.read_fname()?;

        let _package_name = if ue4 >= 369 && (ue4 >= 1000) {
            Some(archive.read_fname()?)
        } else {
            None
        };

        let _import_optional = if ue4 >= 1000 {
            archive.read_u32()? != 0
        } else {
            false
        };

        imports.push(ObjectImport {
            class_package,
            class_name,
            outer_index,
            object_name,
        });
    }
    Ok(imports)
}
