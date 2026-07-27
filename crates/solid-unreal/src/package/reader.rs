use std::io::SeekFrom;
use std::path::Path;

use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::package::import_export;
use crate::package::name_table;
use crate::package::summary::PackageFileSummary;
use crate::types::{FName, FNameEntry, ObjectExport, ObjectImport, PackageIndex};
use crate::version::PackageVersion;

/// A fully parsed UE package file with resolved tables.
///
/// This is the high-level entry point for accessing a package's contents.
/// It owns the name table and can resolve `FName` and `FPackageIndex` values
/// to their string representations.
#[derive(Debug, Clone)]
pub struct UPackage {
    /// Parsed file summary.
    pub summary: PackageFileSummary,
    /// Resolved name table.
    pub names: Vec<FNameEntry>,
    /// Import table.
    pub imports: Vec<ObjectImport>,
    /// Export table.
    pub exports: Vec<ObjectExport>,
    /// File path (if loaded from a file).
    pub file_path: Option<std::path::PathBuf>,
    /// Package version information.
    pub version: PackageVersion,
    /// Index of "None" in the name table (may differ from 0 in cooked packages).
    pub none_name_index: i32,
}

impl UPackage {
    /// Parse a UE package from a reader.
    pub fn read(
        reader: &mut (dyn solid_rs::traits::ReadSeek),
        _file_length: u64,
    ) -> Result<Self, UnrealError> {
        let mut archive = FArchiveUE::new(reader, PackageVersion::new(0, 0));
        let summary = PackageFileSummary::read(&mut archive)?;
        let ver = archive.version().clone();
        drop(archive);

        let fl = _file_length;

        let names = {
            let mut archive = FArchiveUE::new(reader, ver.clone());

            if summary.name_offset > 0 && (summary.name_offset as u64) < fl {
                archive.seek_to(summary.name_offset as u64)?;
                name_table::read_name_table(&mut archive, summary.name_count)?
            } else {
                Vec::new()
            }
        };

        let imports = if summary.import_count > 0 && (summary.import_offset as u64) < fl {
            let import_offset = summary.import_offset as u64;
            let mut archive = FArchiveUE::new(reader, ver.clone());
            archive.seek_to(import_offset)?;
            import_export::read_import_table(&mut archive, summary.import_count)?
        } else {
            Vec::new()
        };

        let exports = if summary.export_count > 0 && (summary.export_offset as u64) < fl {
            let export_offset = summary.export_offset as u64;
            let mut archive = FArchiveUE::new(reader, ver.clone());
            archive.seek_to(export_offset)?;
            import_export::read_export_table(&mut archive, summary.export_count)?
        } else {
            Vec::new()
        };

        // Find "None" in the name table (may not be at index 0 in cooked packages)
        let none_idx = names.iter().position(|n| n.text == "None")
            .map(|i| i as i32)
            .unwrap_or(0);

        Ok(Self {
            summary,
            names,
            imports,
            exports,
            file_path: None,
            version: ver,
            none_name_index: none_idx,
        })
    }

    /// Parse a UE package from a file.
    pub fn read_file(path: impl AsRef<Path>) -> Result<Self, UnrealError> {
        let path = path.as_ref();
        let file_length = std::fs::metadata(path)?.len();
        let mut file = std::fs::File::open(path)?;
        let mut pkg = Self::read(&mut file, file_length)?;
        pkg.file_path = Some(path.to_path_buf());
        Ok(pkg)
    }

    // ── Name resolution ──────────────────────────────────────────────────

    /// Resolve an `FName` to its display string.
    pub fn resolve_name(&self, name: FName) -> String {
        let idx = name.index as usize;
        if idx < self.names.len() {
            let base = &self.names[idx].text;
            if name.number > 0 {
                format!("{}_{}", base, name.number)
            } else {
                base.clone()
            }
        } else {
            format!("<invalid_name_index {}>", name.index)
        }
    }

    /// Resolve an `FPackageIndex` to either an import or an export name.
    pub fn resolve_index(&self, index: PackageIndex) -> Option<String> {
        if index.is_null() {
            return None;
        }
        if index.is_export() {
            let idx = (index.0 - 1) as usize;
            self.exports.get(idx).map(|e| self.resolve_name(e.object_name))
        } else {
            let idx = ((-index.0) - 1) as usize;
            self.imports.get(idx).map(|i| self.resolve_name(i.object_name))
        }
    }

    /// Create a PropertyReader for this package with correct None-name handling.
    pub fn property_reader<'a>(
        &'a self,
        archive: crate::archive::FArchiveUE<'a>,
    ) -> crate::uobject::property::PropertyReader<'a> {
        crate::uobject::property::PropertyReader::new_with_none(
            archive,
            &self.names,
            &self.imports,
            &self.exports,
            self.none_name_index,
        )
    }

    /// Create an archive positioned at the start of a specific export's serial data.
    pub fn archive_for_export<'a>(
        &self,
        reader: &'a mut (dyn solid_rs::traits::ReadSeek),
        export_idx: usize,
    ) -> Result<FArchiveUE<'a>, UnrealError> {
        let export = self.exports.get(export_idx).ok_or_else(|| {
            UnrealError::Parse {
                context: "archive_for_export",
                detail: format!("export index {export_idx} out of range ({} exports)", self.exports.len()),
            }
        })?;

        let offset = if self.version.is_ue5() {
            export.serial_offset as u64
        } else {
            // UE4 offset is from the start of the file
            export.serial_offset as u64
        };

        reader.seek(SeekFrom::Start(offset))?;

        Ok(FArchiveUE::new(reader, self.version.clone()))
    }
}
