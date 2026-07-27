use std::fmt;
use std::io;

use solid_rs::error::SolidError;

#[derive(Debug)]
pub enum UnrealError {
    Io(io::Error),
    Uasset(uasset::Error),
    Parse { context: &'static str, detail: String },
    Conversion { asset_type: &'static str, detail: String },
    Decompress { method: &'static str, detail: String },
    UnresolvedReference { index: i32, context: &'static str },
}

impl fmt::Display for UnrealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Uasset(e) => write!(f, "uasset error: {e}"),
            Self::Parse { context, detail } => {
                write!(f, "parse error at {context}: {detail}")
            }
            Self::Conversion { asset_type, detail } => {
                write!(f, "conversion of {asset_type} failed: {detail}")
            }
            Self::Decompress { method, detail } => {
                write!(f, "decompression ({method}) error: {detail}")
            }
            Self::UnresolvedReference { index, context } => {
                write!(f, "unresolved FPackageIndex({index}) in {context}")
            }
        }
    }
}

impl std::error::Error for UnrealError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Uasset(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for UnrealError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<uasset::Error> for UnrealError {
    fn from(e: uasset::Error) -> Self {
        Self::Uasset(e)
    }
}

impl From<UnrealError> for SolidError {
    fn from(e: UnrealError) -> Self {
        SolidError::parse(e.to_string())
    }
}
