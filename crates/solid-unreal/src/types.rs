/// UE package magic number (big-endian).
pub const PACKAGE_FILE_MAGIC: u32 = 0x9E2A_83C1;

/// An index into either the import or export table.
///
/// * `0` → null (no reference).
/// * positive → `index - 1` into export table.
/// * negative → `(-index) - 1` into import table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackageIndex(pub i32);

impl PackageIndex {
    pub const NULL: Self = Self(0);

    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn is_export(self) -> bool {
        self.0 > 0
    }

    pub fn is_import(self) -> bool {
        self.0 < 0
    }

    pub fn to_index(self) -> Option<usize> {
        if self.0 > 0 {
            Some((self.0 - 1) as usize)
        } else if self.0 < 0 {
            Some(((-self.0) - 1) as usize)
        } else {
            None
        }
    }
}

/// A name table entry (`FNameEntry`).
#[derive(Debug, Clone)]
pub struct FNameEntry {
    /// The string value (may be precomputed hash in UE5).
    pub text: String,
    /// Precomputed hash (UE5+).
    pub hash: Option<u64>,
    /// Whether this entry uses a non-ansi encoding.
    pub is_wide: bool,
}

/// A name table reference (index + number suffix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FName {
    /// Index into the name table (`FNameEntry` array).
    pub index: i32,
    /// Suffix number (e.g. `"Name_2"` → number=2).
    pub number: i32,
}

impl FName {
    pub fn new(index: i32, number: i32) -> Self {
        Self { index, number }
    }
}

bitflags::bitflags! {
    /// Flags stored in `FPackageFileSummary.package_flags`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PackageFlags: u32 {
        const NEWLY_CREATED     = 0x0000_0001;
        const CLIENT_OPTIONAL   = 0x0000_0002;
        const SERVER_SIDE_ONLY  = 0x0000_0004;
        const COMPILED_INTO_CHIN = 0x0000_0010;
        const HAS_FLAGS         = 0x0000_0020;
        const PAYLOAD           = 0x0000_0040;
        const NO_EXPORT_INFO    = 0x0000_0080;
        const UNUSED_9          = 0x0000_0100;
        const UNUSED_10         = 0x0000_0200;
        const COMPRESSED        = 0x0000_0400;
        const UNCOMPRESSED      = 0x0000_1000;
        const UNUSED_13         = 0x0000_2000;
        const UNUSED_14         = 0x0000_4000;
        const UNUSED_15         = 0x0000_8000;
        const NEEDS_SUMMARIZATION = 0x0001_0000;
        const COMPRESSION       = 0x0002_0000;
        const MAP               = 0x0004_0000;
        const PLAYER_SAVE       = 0x0008_0000;
        const STORE_COMPRESSED_ENCRYPTED = 0x0010_0000;
        const UNUSED_21         = 0x0020_0000;
        const STORE_STRIPED     = 0x0040_0000;
        const CONTAINING_MAP    = 0x0080_0000;
        const COOKED            = 0x0100_0000;
        const TRASHED           = 0x0200_0000;
        const PROTECTED         = 0x0400_0000;
        const UNUSED_27         = 0x0800_0000;
        const UNUSED_28         = 0x1000_0000;
        const SCRIPT           = 0x2000_0000;
        const UNUSED_30        = 0x4000_0000;
        const UNUSED_31        = 0x8000_0000;
        /// `PKG_FilterEditorOnly` — cooked packages have this flag set.
        const FILTER_EDITOR_ONLY = 0x0000_8000;
    }
}

/// Compression methods used in UE packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMethod {
    Zlib,
    LZ4,
    Oodle,
}

/// Describes a single compressed block within the package.
#[derive(Debug, Clone)]
pub struct CompressedBlock {
    pub uncompressed_offset: i64,
    pub uncompressed_size: i64,
    pub compressed_offset: i64,
    pub compressed_size: i64,
}

/// Describes how a package region is compressed.
#[derive(Debug, Clone)]
pub struct CompressionInfo {
    pub method: CompressionMethod,
    pub block_size: i32,
    pub blocks: Vec<CompressedBlock>,
}

/// Flags for `FObjectExport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFlags {
    None = 0,
    ForcedExport = 1,
    NotForClient = 2,
    NotForServer = 4,
    NotAlwaysLoadedForEditor = 8,
    HasAllFlags = 16,
}

/// An entry in the export table.
#[derive(Debug, Clone)]
pub struct ObjectExport {
    pub class_index: PackageIndex,
    pub outer_index: PackageIndex,
    pub object_name: FName,
    pub object_flags: u32,
    pub serial_size: i64,
    pub serial_offset: i64,
    pub package_flags: u32,
    pub export_flags: u8,
    /// UE5+ generic header extension data.
    pub header_extensions: Vec<(FName, Vec<u8>)>,
}

/// An entry in the import table.
#[derive(Debug, Clone)]
pub struct ObjectImport {
    pub class_package: FName,
    pub class_name: FName,
    pub outer_index: PackageIndex,
    pub object_name: FName,
}

impl Default for ObjectExport {
    fn default() -> Self {
        Self {
            class_index: PackageIndex::NULL,
            outer_index: PackageIndex::NULL,
            object_name: FName::new(0, 0),
            object_flags: 0,
            serial_size: 0,
            serial_offset: 0,
            package_flags: 0,
            export_flags: 0,
            header_extensions: Vec::new(),
        }
    }
}

impl Default for ObjectImport {
    fn default() -> Self {
        Self {
            class_package: FName::new(0, 0),
            class_name: FName::new(0, 0),
            outer_index: PackageIndex::NULL,
            object_name: FName::new(0, 0),
        }
    }
}
