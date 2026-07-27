/// UE engine version triples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EngineVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl EngineVersion {
    pub const UE_4_0: Self = Self { major: 4, minor: 0, patch: 0 };
    pub const UE_4_27: Self = Self { major: 4, minor: 27, patch: 0 };
    pub const UE_5_0: Self = Self { major: 5, minor: 0, patch: 0 };
    pub const UE_5_1: Self = Self { major: 5, minor: 1, patch: 0 };
    pub const UE_5_2: Self = Self { major: 5, minor: 2, patch: 0 };
    pub const UE_5_3: Self = Self { major: 5, minor: 3, patch: 0 };
    pub const UE_5_4: Self = Self { major: 5, minor: 4, patch: 0 };
    pub const UE_5_5: Self = Self { major: 5, minor: 5, patch: 0 };

    pub fn as_u32(&self) -> u32 {
        (self.major as u32) * 100_00 + (self.minor as u32) * 100 + self.patch as u32
    }

    pub fn from_u32(v: u32) -> Self {
        Self {
            major: (v / 100_00) as u16,
            minor: ((v % 100_00) / 100) as u16,
            patch: (v % 100) as u16,
        }
    }
}

impl std::fmt::Display for EngineVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Known legacy UE4 package versions (`VER_UE4_*`).
///
/// These are the last ~20 values that cover the main format transitions.
/// For a complete list, see UE source `CoreUObject/Public/UObject/ObjectVersion.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UE4Version {
    // --- Major feature versions (ascending) ---
    VerNewLightmass = 19,
    VerRemovedDependencyLoadOnDemand = 23,
    VerBlueprintsUseSparseClassData = 31,
    VerFontOutline = 39,
    VerWorldComposition = 41,
    VerSkeletonGuid = 42,
    VerAnimAdditive = 49,
    VerStringAssetReferences = 53,
    VerSkipOnlyEditorCustomer = 57,
    Ver4_15 = 62,
    Ver4_16 = 63,
    Ver4_17 = 64,
    Ver4_18 = 65,
    Ver4_19 = 66,
    Ver4_20 = 67,
    Ver4_21 = 68,
    Ver4_22 = 69,
    Ver4_23 = 70,
    Ver4_24 = 71,
    Ver4_25 = 72,
    Ver4_26 = 73,
    Ver4_27 = 74,
    /// Maximum UE4 version (the last before UE5 locks it).
    Assume16BitPackageGuid = 75,
}

impl UE4Version {
    pub fn from_i32(v: i32) -> Option<Self> {
        Some(match v {
            19 => Self::VerNewLightmass,
            23 => Self::VerRemovedDependencyLoadOnDemand,
            31 => Self::VerBlueprintsUseSparseClassData,
            39 => Self::VerFontOutline,
            41 => Self::VerWorldComposition,
            42 => Self::VerSkeletonGuid,
            49 => Self::VerAnimAdditive,
            53 => Self::VerStringAssetReferences,
            57 => Self::VerSkipOnlyEditorCustomer,
            62 => Self::Ver4_15,
            63 => Self::Ver4_16,
            64 => Self::Ver4_17,
            65 => Self::Ver4_18,
            66 => Self::Ver4_19,
            67 => Self::Ver4_20,
            68 => Self::Ver4_21,
            69 => Self::Ver4_22,
            70 => Self::Ver4_23,
            71 => Self::Ver4_24,
            72 => Self::Ver4_25,
            73 => Self::Ver4_26,
            74 => Self::Ver4_27,
            75 => Self::Assume16BitPackageGuid,
            _ => return None,
        })
    }
}

/// Package version information detected from the file.
#[derive(Debug, Clone, Copy)]
pub struct PackageVersion {
    /// Legacy file version (negative for modern UE4/5 cooked packages).
    pub legacy: i32,
    /// FileVersionUE4 — the raw UE4 version value from the package header.
    pub ue4_raw: i32,
    /// Engine version triple (e.g. UE 5.3 → `5.3.0`).
    pub engine: EngineVersion,
    /// Companion UE4 version enum, if the value is known.
    pub ue4: Option<UE4Version>,
}

impl PackageVersion {
    pub fn new(legacy: i32, engine: u32) -> Self {
        Self {
            legacy,
            ue4_raw: 0,
            engine: EngineVersion::from_u32(engine),
            ue4: UE4Version::from_i32(legacy),
        }
    }

    /// Create a PackageVersion with explicit raw UE4 version value.
    pub fn with_ue4(legacy: i32, ue4_raw: i32, ue5_major: u16) -> Self {
        Self {
            legacy,
            ue4_raw,
            engine: if ue5_major >= 5 {
                EngineVersion { major: ue5_major, minor: 0, patch: 0 }
            } else {
                EngineVersion { major: 4, minor: 27, patch: 0 }
            },
            ue4: UE4Version::from_i32(ue4_raw),
        }
    }

    /// Is this a UE5+ package?
    pub fn is_ue5(&self) -> bool {
        self.engine.major >= 5 || self.ue4_raw >= 1000
    }

    /// Is this a UE4 package?
    pub fn is_ue4(&self) -> bool {
        self.engine.major == 4 || (!self.is_ue5() && self.ue4.is_some())
    }

    /// Is this a cooked package (negative legacy version)?
    pub fn is_cooked(&self) -> bool {
        self.legacy < 0
    }
}
