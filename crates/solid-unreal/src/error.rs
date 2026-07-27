use std::fmt;
use std::io;

use solid_rs::error::SolidError;

/// Errors specific to Unreal package parsing.
#[derive(Debug)]
pub enum UnrealError {
    Io(io::Error),
    /// The file is not a valid UE package (bad magic).
    BadMagic { found: u32 },
    /// The UE package version is not supported.
    UnsupportedVersion { legacy: i32, engine: u32 },
    /// An expected value was not found during parsing.
    Parse {
        context: &'static str,
        detail: String,
    },
    /// A referenced object could not be resolved.
    UnresolvedReference {
        index: i32,
        context: &'static str,
    },
    /// Decompression of package data failed.
    Decompress {
        method: &'static str,
        detail: String,
    },
    /// Bulk data was not found or could not be read.
    BulkData {
        filename: String,
        detail: String,
    },
    /// A conversion step failed (mesh, material, etc.).
    Conversion {
        asset_type: &'static str,
        detail: String,
    },
}

impl fmt::Display for UnrealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::BadMagic { found } => {
                write!(f, "bad UE package magic: 0x{found:08X}, expected 0x9E2A83C1")
            }
            Self::UnsupportedVersion { legacy, engine } => {
                write!(f, "unsupported UE version: legacy={legacy}, engine={engine}")
            }
            Self::Parse { context, detail } => {
                write!(f, "parse error at {context}: {detail}")
            }
            Self::UnresolvedReference { index, context } => {
                write!(f, "unresolved FPackageIndex({index}) in {context}")
            }
            Self::Decompress { method, detail } => {
                write!(f, "decompression ({method}) error: {detail}")
            }
            Self::BulkData { filename, detail } => {
                write!(f, "bulk data file '{filename}': {detail}")
            }
            Self::Conversion { asset_type, detail } => {
                write!(f, "conversion of {asset_type} failed: {detail}")
            }
        }
    }
}

impl std::error::Error for UnrealError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for UnrealError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<UnrealError> for SolidError {
    fn from(e: UnrealError) -> Self {
        SolidError::parse(e.to_string())
    }
}
