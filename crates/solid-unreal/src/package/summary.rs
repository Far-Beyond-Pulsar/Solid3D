use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::types::PackageFlags;
use crate::version::PackageVersion;

/// FPackageFileSummary reader.
#[derive(Debug, Clone)]
pub struct PackageFileSummary {
    pub tag: u32,
    pub legacy_version: i32,
    pub ue3_version: i32,
    pub ue4_version: i32,
    pub ue5_version: i32,
    pub licensee_version: i32,
    pub package_flags: PackageFlags,
    pub name_count: i32,
    pub name_offset: i32,
    pub export_count: i32,
    pub export_offset: i32,
    pub import_count: i32,
    pub import_offset: i32,
    pub total_header_size: i32,
}

impl PackageFileSummary {
    pub fn read(archive: &mut FArchiveUE) -> Result<Self, UnrealError> {
        let tag = archive.read_u32()?;
        if tag != crate::types::PACKAGE_FILE_MAGIC {
            return Err(UnrealError::BadMagic { found: tag });
        }
        let legacy_version = archive.read_i32()?;

        let (ue3, ue4, ue5, licensee) = if legacy_version < 0 {
            let u3 = if legacy_version != -4 { archive.read_i32()? } else { 0 };
            let u4 = archive.read_i32()?;
            let u5 = if legacy_version <= -8 { archive.read_i32()? } else { 0 };
            let lic = archive.read_i32()?;
            (u3, u4, u5, lic)
        } else {
            (archive.read_i32()?, archive.read_i32()?, 0, archive.read_i32()?)
        };

        let ue5_major = if ue5 > 0 { (ue5 / 100) as u16 } else { 0 };
        let ver = PackageVersion::with_ue4(legacy_version, ue4, ue5_major);
        *archive.version_mut() = ver;

        if legacy_version < 0 {
            return Self::read_cooked_minimal(archive, tag, legacy_version, ue3, ue4, ue5, licensee);
        }

        // === Uncooked (unchanged) ===
        if ue4 >= 339 && ue4 <= 522 {
            let _ = archive.read_i32()?; let _ = archive.read_i32()?;
        }
        if ue5 >= 1017 { let _ = archive.read_guid()?; }
        let _ = archive.read_serial_size()?;
        if ue3 >= 249 && ue5 < 1017 { let _ = archive.read_i32()?; }
        if ue3 >= 269 { let _ = archive.read_fstring()?; }

        let pf = archive.read_u32()?;
        let pkg = PackageFlags::from_bits_truncate(pf);
        let filt = (pf & 0x8000_0000) != 0;
        let nc = archive.read_i32()?;
        let no = archive.read_i32()?;
        if ue5 >= 1008 { let _ = archive.read_i32()?; let _ = archive.read_i32()?; }
        if !filt && ue4 >= 372 { let _ = archive.read_fstring()?; }
        if ue4 >= 370 { let _ = archive.read_i32()?; let _ = archive.read_i32()?; }
        let ec = archive.read_i32()?;
        let eo = archive.read_i32()?;
        let ic = archive.read_i32()?;
        let io = archive.read_i32()?;

        return Ok(PackageFileSummary {
            tag, legacy_version, ue3_version: ue3, ue4_version: ue4,
            ue5_version: ue5, licensee_version: licensee,
            package_flags: pkg,
            name_count: nc, name_offset: no,
            export_count: ec, export_offset: eo,
            import_count: ic, import_offset: io,
            total_header_size: 0,
        });
    }

    /// Minimal cooked reader: reads only the fields needed to locate
    /// name/export/import tables, then stops.
    fn read_cooked_minimal(
        archive: &mut FArchiveUE,
        tag: u32,
        legacy_version: i32,
        ue3: i32,
        ue4: i32,
        ue5: i32,
        licensee: i32,
    ) -> Result<Self, UnrealError> {
        // READD_COOKER: CUE4Parse uses re-indexed values (138, 142).
        // ue4=516 > 142 → skip.
        if ue4 >= 138 && ue4 <= 142 {
            let _ = archive.read_i32()?;
            let _ = archive.read_i32()?;
        }

        // Custom versions (9 entries for this file)
        let cvc = archive.read_i32()?;
        for _ in 0..cvc {
            let _ = archive.read_guid()?;
            let _ = archive.read_i32()?;
        }

        // TotalHeaderSize
        if ue3 >= 249 && ue5 < 1017 {
            let _ = archive.read_i32()?;
        }

        // PackageName
        if ue3 >= 269 { let _pn = archive.read_fstring()?; drop(_pn); }

        let pf = archive.read_u32()?;
        let is_filt = (pf & 0x8000) != 0 || (pf & 0x8000_0000) != 0;

        let nc = archive.read_i32()?;
        let no = archive.read_i32()?;

        // SoftObjectPath (UE5)
        if ue5 >= 1008 { let _ = archive.read_i32()?; let _ = archive.read_i32()?; }

        // LocalizationId (UE4.12+: VER_UE4_ADDED_PACKAGE_SUMMARY_LOCALIZATION_ID = 372)
        // Present in this cooked file with ue4=516.
        if !is_filt && ue4 >= 372 { let _ = archive.read_fstring()?; }

        // GatherableTextData (UE4.12+: VER_UE4_SERIALIZE_TEXT_IN_PACKAGES = 370)
        if ue4 >= 370 { let _ = archive.read_i32()?; let _ = archive.read_i32()?; }

        // Export/Import tables
        let ec = archive.read_i32()?;
        let eo = archive.read_i32()?;
        let ic = archive.read_i32()?;
        let io = archive.read_i32()?;

        // HeritageTable (UE3 DeprecatedHeritageTable = 68)
        if ue3 < 68 { let _ = archive.read_i32()?; let _ = archive.read_i32()?; }

        // Cell/CellExport/Import (UE5): skip

        // MetaDataOffset (UE5): skip

        // DependsOffset (UE3 ADDED_LINKER_DEPENDENCIES = 415)
        if ue3 >= 415 { let _ = archive.read_i32()?; }

        // SoftPackageReferences (UE4: CUE4Parse re-indexed, skip for cooked)
        // SearchableNamesOffset (UE4: CUE4Parse re-indexed, skip for cooked)

        // ThumbnailTableOffset (UE3 ASSET_THUMBNAILS_IN_PACKAGES = 377)
        if ue3 >= 377 { let _ = archive.read_i32()?; }

        // Guid (always present for UE4/UE5 < PACKAGE_SAVED_HASH)
        if ue5 < 1017 { let _ = archive.read_guid()?; }

        // PersistentGuid + ownerPersistentGuid (CUE4Parse re-indexed: ADDED_PACKAGE_OWNER)
        // CUE4Parse value ≈ 835, which is > ue4=516, so SKIP for this file.

        // Generations (CUE4Parse re-indexed: skip for this file)

        // EngineVersion (CUE4Parse re-indexed: skip for this file)
        // CompatibleEngineVersion (skip)
        // CompressionFlags (skip)
        // CompressedChunks (skip)
        // PackageSource (skip)
        // AdditionalPackagesToCook (skip)
        // AssetRegistryDataOffset (skip)
        // BulkDataStartOffset (skip)
        // WorldTileInfoDataOffset (skip)
        // ChunkIds (skip)
        // PreloadDependencies (skip)

        // We've read enough. The remaining summary fields are not needed
        // for the name/export/import tables.

        Ok(PackageFileSummary {
            tag, legacy_version, ue3_version: ue3, ue4_version: ue4,
            ue5_version: ue5, licensee_version: licensee,
            package_flags: PackageFlags::from_bits_truncate(pf),
            name_count: nc, name_offset: no,
            export_count: ec, export_offset: eo,
            import_count: ic, import_offset: io,
            total_header_size: 0,
        })
    }
}