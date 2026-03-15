//! PLY header parser.

use solid_rs::{Result, SolidError};

#[derive(Debug, Clone, PartialEq)]
pub enum PlyFormat {
    Ascii,
    BinaryLE,
    BinaryBE,
}

#[derive(Debug, Clone)]
pub enum PropType {
    Scalar(ScalarType),
    List { count_type: ScalarType, value_type: ScalarType },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScalarType {
    I8, U8, I16, U16, I32, U32, F32, F64,
}

impl ScalarType {
    pub fn byte_size(self) -> usize {
        match self {
            ScalarType::I8  | ScalarType::U8  => 1,
            ScalarType::I16 | ScalarType::U16 => 2,
            ScalarType::I32 | ScalarType::U32 | ScalarType::F32 => 4,
            ScalarType::F64 => 8,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "char"   | "int8"    => Some(Self::I8),
            "uchar"  | "uint8"   => Some(Self::U8),
            "short"  | "int16"   => Some(Self::I16),
            "ushort" | "uint16"  => Some(Self::U16),
            "int"    | "int32"   => Some(Self::I32),
            "uint"   | "uint32"  => Some(Self::U32),
            "float"  | "float32" => Some(Self::F32),
            "double" | "float64" => Some(Self::F64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Property {
    pub name:      String,
    pub prop_type: PropType,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub name:       String,
    pub count:      usize,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone)]
pub struct PlyHeader {
    pub format:          PlyFormat,
    pub elements:        Vec<Element>,
    /// Byte offset at which the data section begins.
    pub header_byte_len: usize,
    pub comments:        Vec<String>,
}

pub fn parse_header(data: &[u8]) -> Result<PlyHeader> {
    let end_marker = b"end_header";
    let header_end = data.windows(end_marker.len())
        .position(|w| w == end_marker)
        .ok_or_else(|| SolidError::parse("PLY: missing end_header"))?;

    let header_text = std::str::from_utf8(&data[..header_end])
        .map_err(|_| SolidError::parse("PLY: header is not valid UTF-8"))?;

    // Skip past "end_header" and the newline(s) that follow.
    let mut data_start = header_end + end_marker.len();
    if data.get(data_start).copied() == Some(b'\r') {
        data_start += 1;
    }
    if data.get(data_start).copied() == Some(b'\n') {
        data_start += 1;
    }

    let mut format   = PlyFormat::Ascii;
    let mut elements: Vec<Element> = Vec::new();
    let mut comments: Vec<String>  = Vec::new();

    for line in header_text.lines() {
        let line  = line.trim();
        if line.is_empty() || line == "ply" { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "format" => {
                format = match parts.get(1).copied() {
                    Some("ascii")                => PlyFormat::Ascii,
                    Some("binary_little_endian") => PlyFormat::BinaryLE,
                    Some("binary_big_endian")    => PlyFormat::BinaryBE,
                    other => return Err(SolidError::parse(format!(
                        "PLY: unknown format '{}'", other.unwrap_or("?")
                    ))),
                };
            }
            "comment" => {
                comments.push(line[7..].trim().to_string());
            }
            "element" => {
                let name  = parts.get(1).unwrap_or(&"unknown").to_string();
                let count: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                elements.push(Element { name, count, properties: Vec::new() });
            }
            "property" => {
                if let Some(elem) = elements.last_mut() {
                    if parts.get(1).copied() == Some("list") {
                        let ct = parts.get(2)
                            .and_then(|s| ScalarType::from_str(s))
                            .ok_or_else(|| SolidError::parse("PLY: invalid list count type"))?;
                        let vt = parts.get(3)
                            .and_then(|s| ScalarType::from_str(s))
                            .ok_or_else(|| SolidError::parse("PLY: invalid list value type"))?;
                        let name = parts.get(4).unwrap_or(&"unknown").to_string();
                        elem.properties.push(Property {
                            name,
                            prop_type: PropType::List { count_type: ct, value_type: vt },
                        });
                    } else {
                        let scalar_type = parts.get(1)
                            .and_then(|s| ScalarType::from_str(s))
                            .ok_or_else(|| SolidError::parse(format!(
                                "PLY: unknown scalar type '{}'",
                                parts.get(1).unwrap_or(&"?")
                            )))?;
                        let name = parts.get(2).unwrap_or(&"unknown").to_string();
                        elem.properties.push(Property {
                            name,
                            prop_type: PropType::Scalar(scalar_type),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    Ok(PlyHeader { format, elements, header_byte_len: data_start, comments })
}
