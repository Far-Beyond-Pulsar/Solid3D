use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::types::FName;

/// Represents a serialized UE property tag.
///
/// Each serialized property in a UObject starts with a tag that identifies
/// the type, name, and size of the following data.
#[derive(Debug, Clone)]
pub struct PropertyTag {
    pub name: FName,
    pub type_name: FName,
    pub size: i32,
    pub array_index: i32,
    /// For struct properties, the struct type name.
    pub struct_name: FName,
    /// Next property tag offset (for skipping).
    pub next_offset: u64,
    /// Whether this is the last property (None type marker).
    pub is_none: bool,
}

/// Reads a property tag from the archive.
///
/// Returns `None` when the None terminator is reached (name == NAME_None).
pub fn read_property_tag(
    archive: &mut FArchiveUE,
) -> Result<Option<PropertyTag>, UnrealError> {
    let name = archive.read_fname()?;

    // NAME_None terminates property serialization
    if name.index == 0 && name.number == 0 {
        return Ok(None);
    }

    let type_name = archive.read_fname()?;
    let size = archive.read_i32()?;
    let array_index = archive.read_i32()?;

    // After size and array_index, advance to the next property for offset tracking
    let _after_header = archive.pos;

    // If the property is a struct, read the struct name and skip the rest of the tag
    let is_struct = resolve_name_text(archive, type_name) == "StructProperty";
    let struct_name = if is_struct {
        let sname = archive.read_fname()?;
        // Skip the rest of the tag: struct_guid(16) + unknown
        archive.skip(20)?; // 16 bytes GUID + 4 bytes padding
        sname
    } else {
        // Check for other property types that have extra tag data
        let type_text = resolve_name_text(archive, type_name);
        match type_text.as_str() {
            "BoolProperty" => {
                // BoolProperty has a 1-byte bool value in the tag itself
                // (already read as part of the property data)
            }
            "ArrayProperty" | "SetProperty" => {
                let _inner = archive.read_fname()?;
            }
            "MapProperty" => {
                let _key = archive.read_fname()?;
                let _val = archive.read_fname()?;
            }
            "ByteProperty" | "EnumProperty" => {
                // Read the enum name
                let _enum_name = archive.read_fname()?;
            }
            "ObjectProperty" | "ObjectPtrProperty" => {
                // Read the property class reference
                let _class_name = archive.read_fname()?;
            }
            "InterfaceProperty" => {
                let _interface_name = archive.read_fname()?;
            }
            "FieldPathProperty" => {
                let _field_class = archive.read_fname()?;
            }
            _ => {}
        }
        FName::new(0, 0)
    };

    let next_offset = archive.pos;

    Ok(Some(PropertyTag {
        name,
        type_name,
        size,
        array_index,
        struct_name,
        next_offset,
        is_none: false,
    }))
}

fn resolve_name_text(_archive: &FArchiveUE, _name: FName) -> String {
    // We don't have access to the name table here.
    // In practice, this is handled by passing the UPackage to the property reader.
    // For now, return a placeholder; the actual implementation will use the package's name table.
    format!("Name({})", _name.index)
}

/// A property reader that has access to the package name table.
pub struct PropertyReader<'a> {
    archive: FArchiveUE<'a>,
    names: &'a [crate::types::FNameEntry],
    imports: &'a [crate::types::ObjectImport],
    exports: &'a [crate::types::ObjectExport],
    /// Index of "None" in the name table (may differ from 0 in cooked packages).
    none_name_index: i32,
}

impl<'a> PropertyReader<'a> {
    pub fn new(
        archive: FArchiveUE<'a>,
        names: &'a [crate::types::FNameEntry],
        imports: &'a [crate::types::ObjectImport],
        exports: &'a [crate::types::ObjectExport],
    ) -> Self {
        Self { archive, names, imports, exports, none_name_index: 0 }
    }

    /// Create a PropertyReader with a custom "None" name index (for cooked packages).
    pub fn new_with_none(
        archive: FArchiveUE<'a>,
        names: &'a [crate::types::FNameEntry],
        imports: &'a [crate::types::ObjectImport],
        exports: &'a [crate::types::ObjectExport],
        none_name_index: i32,
    ) -> Self {
        Self { archive, names, imports, exports, none_name_index }
    }

    pub fn archive(&mut self) -> &mut FArchiveUE<'a> {
        &mut self.archive
    }

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

    /// Skip all properties until the None terminator.
    pub fn skip_remaining_properties(&mut self) -> Result<(), UnrealError> {
        loop {
            let tag = read_property_tag(&mut self.archive)?;
            match tag {
                None => return Ok(()),
                Some(tag) => {
                    self.archive.seek_to(tag.next_offset)?;
                }
            }
        }
    }

    /// Read the next property tag (with name resolution).
    pub fn read_tag(&mut self) -> Result<Option<PropertyTagInfo>, UnrealError> {
        let tag_opt = self.read_property_tag_internal()?;
        match tag_opt {
            None => Ok(None),
            Some(tag) => {
                let name_str = self.resolve_name(tag.name);
                let type_str = self.resolve_name(tag.type_name);
                let struct_str = self.resolve_name(tag.struct_name);
                Ok(Some(PropertyTagInfo {
                    raw: tag,
                    name: name_str,
                    type_name: type_str,
                    struct_name: struct_str,
                }))
            }
        }
    }

    fn read_property_tag_internal(
        &mut self,
    ) -> Result<Option<crate::uobject::property::PropertyTag>, UnrealError> {
        let name = self.archive.read_fname()?;

        // NAME_None terminates property stream; use the correct index
        // (may differ from 0 in cooked packages)
        if name.index == self.none_name_index && name.number == 0 {
            return Ok(None);
        }

        let type_name = self.archive.read_fname()?;
        let size = self.archive.read_i32()?;
        let array_index = self.archive.read_i32()?;

        let type_text = self.resolve_name(type_name);
        let struct_name = if type_text == "StructProperty" {
            let sname = self.archive.read_fname()?;
            self.archive.skip(20)?;
            sname
        } else {
            match type_text.as_str() {
                "BoolProperty" => {}
                "ArrayProperty" | "SetProperty" => {
                    let inner = self.archive.read_fname()?;
                    let inner_name = self.resolve_name(inner);
                    // If inner type is "StructProperty", there's an extra FName for the struct type
                    if inner_name == "StructProperty" {
                        let _struct_type = self.archive.read_fname()?;
                    }
                }
                "MapProperty" => {
                    let _key = self.archive.read_fname()?;
                    let _val = self.archive.read_fname()?;
                }
                "ByteProperty" | "EnumProperty" => {
                    let _ = self.archive.read_fname()?;
                }
                "ObjectProperty" | "ObjectPtrProperty" => {
                    let _ = self.archive.read_fname()?;
                }
                "InterfaceProperty" => {
                    let _ = self.archive.read_fname()?;
                }
                "FieldPathProperty" => {
                    let _ = self.archive.read_fname()?;
                }
                _ => {}
            }
            FName::new(0, 0)
        };

        let next_offset = self.archive.pos;

        Ok(Some(crate::uobject::property::PropertyTag {
            name,
            type_name,
            size,
            array_index,
            struct_name,
            next_offset,
            is_none: false,
        }))
    }

    /// Seek to the data for a specific named property within the current object.
    /// If the property is found, the archive is positioned at the start of its data.
    /// If not found, the archive is positioned past the None terminator.
    pub fn find_property(&mut self, target_name: &str) -> Result<bool, UnrealError> {
        loop {
            let _saved_pos = self.archive.pos;

            match self.read_tag()? {
                None => return Ok(false),
                Some(tag) => {
                    if tag.name == target_name {
                        // Seek back to the start of the property data
                        // (after the tag header, which we've already passed)
                        // The data starts at saved_pos + tag_header_size
                        // But we already read past it; we need to seek to where data starts
                        // Actually, read_tag positions us at the start of the data.
                        // Wait - no, after reading the tag we're at the position
                        // AFTER the tag header, which is the start of the property data.
                        return Ok(true);
                    }
                    // Skip the property data
                    let data_start = self.archive.pos;
                    let target = tag.raw.next_offset;
                    if target > data_start {
                        self.archive.seek_to(target)?;
                    }
                }
            }
        }
    }

    /// Read a sub-array of properties (for ArrayProperty / SetProperty).
    pub fn read_property_array(
        &mut self,
        inner_tag: &PropertyTagInfo,
        count: i32,
    ) -> Result<Vec<Vec<(String, PropertyTagInfo)>>, UnrealError> {
        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut item = Vec::new();
            // For struct arrays, read all properties of the struct
            if inner_tag.struct_name.is_empty() || inner_tag.struct_name == "None" {
                // Simple type array - read a single value
            } else {
                // Struct array - read properties until None
                loop {
                    match self.read_tag()? {
                        None => break,
                        Some(tag) => item.push((tag.name.clone(), tag)),
                    }
                }
            }
            items.push(item);
        }
        Ok(items)
    }
}

/// A resolved property tag with string names.
#[derive(Debug, Clone)]
pub struct PropertyTagInfo {
    pub raw: crate::uobject::property::PropertyTag,
    pub name: String,
    pub type_name: String,
    pub struct_name: String,
}
