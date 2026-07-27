use std::io::{Read, Seek, SeekFrom};

use crate::error::UnrealError;
use crate::reader;

#[derive(Debug, Clone)]
pub struct PropertyTag {
    pub name: uasset::NameReference,
    pub type_name: uasset::NameReference,
    pub size: i32,
    pub array_index: i32,
    pub struct_name: uasset::NameReference,
    pub next_offset: u64,
    pub is_none: bool,
}

pub fn read_property_tag<R: Read + Seek>(
    reader: &mut R,
    names: &[String],
) -> Result<Option<PropertyTag>, UnrealError> {
    let name = reader::read_fname(reader)?;
    let names_empty = names.is_empty();
    let name_str = if names_empty { String::new() } else { reader::resolve_name(names, &name) };
    if !names_empty && name_str == "None" {
        return Ok(None);
    }
    if name.index == 0 && name.number.is_none() && names_empty {
        return Ok(None);
    }

    let type_name = reader::read_fname(reader)?;
    let size = reader::read_i32(reader)?;
    let array_index = reader::read_i32(reader)?;

    let type_str = reader::resolve_name(names, &type_name);
    let struct_name = if type_str == "StructProperty" {
        let sname = reader::read_fname(reader)?;
        reader.seek(SeekFrom::Current(20))?;
        sname
    } else {
        match type_str.as_str() {
            "BoolProperty" => {}
            "ArrayProperty" | "SetProperty" => {
                let _inner = reader::read_fname(reader)?;
            }
            "MapProperty" => {
                let _key = reader::read_fname(reader)?;
                let _val = reader::read_fname(reader)?;
            }
            "ByteProperty" | "EnumProperty" => {
                let _ = reader::read_fname(reader)?;
            }
            "ObjectProperty" | "ObjectPtrProperty" => {
                let _ = reader::read_fname(reader)?;
            }
            "InterfaceProperty" => {
                let _ = reader::read_fname(reader)?;
            }
            "FieldPathProperty" => {
                let _ = reader::read_fname(reader)?;
            }
            _ => {}
        }
        uasset::NameReference { index: 0, number: None }
    };

    let next_offset = match reader.stream_position() {
        Ok(p) => p,
        Err(_) => 0,
    };

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

pub struct PropertyReader<'a> {
    names: &'a [String],
}

impl<'a> PropertyReader<'a> {
    pub fn new(names: &'a [String]) -> Self {
        Self { names }
    }

    pub fn resolve_name(&self, nr: &uasset::NameReference) -> String {
        reader::resolve_name(self.names, nr)
    }

    pub fn read_tag<R: Read + Seek>(&self, reader: &mut R) -> Result<Option<PropertyTagInfo>, UnrealError> {
        let tag_opt = read_property_tag(reader, self.names)?;
        match tag_opt {
            None => Ok(None),
            Some(tag) => {
                let name_str = self.resolve_name(&tag.name);
                let type_str = self.resolve_name(&tag.type_name);
                let struct_str = self.resolve_name(&tag.struct_name);
                Ok(Some(PropertyTagInfo {
                    raw: tag,
                    name: name_str,
                    type_name: type_str,
                    struct_name: struct_str,
                }))
            }
        }
    }

    pub fn skip_remaining<R: Read + Seek>(&self, reader: &mut R) -> Result<(), UnrealError> {
        loop {
            match read_property_tag(reader, self.names)? {
                None => return Ok(()),
                Some(tag) => {
                    reader.seek(SeekFrom::Start(tag.next_offset))?;
                }
            }
        }
    }

    pub fn find_property<R: Read + Seek>(&self, reader: &mut R, target_name: &str) -> Result<bool, UnrealError> {
        loop {
            match self.read_tag(reader)? {
                None => return Ok(false),
                Some(tag) => {
                    if tag.name == target_name {
                        return Ok(true);
                    }
                    let next = tag.raw.next_offset;
                    let pos = match reader.stream_position() {
                        Ok(p) => p,
                        Err(_) => 0,
                    };
                    if next > pos {
                        reader.seek(SeekFrom::Start(next))?;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyTagInfo {
    pub raw: PropertyTag,
    pub name: String,
    pub type_name: String,
    pub struct_name: String,
}
