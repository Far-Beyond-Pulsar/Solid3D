use std::io::{Seek, SeekFrom};

use glam::{Vec3, Vec4};

use crate::error::UnrealError;
use crate::reader;
use crate::uobject::property::PropertyReader;

#[derive(Debug, Clone)]
pub struct MaterialAsset {
    pub name: String,
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub specular: f32,
    pub emissive_color: Vec3,
    pub opacity: f32,
    pub opacity_mask_clip_value: f32,
    pub blend_mode: u32,
    pub shading_model: u32,
    pub two_sided: bool,
    pub textures: MaterialTextures,
}

#[derive(Debug, Clone, Default)]
pub struct MaterialTextures {
    pub base_color: Option<TextureSlot>,
    pub metallic_roughness: Option<TextureSlot>,
    pub normal: Option<TextureSlot>,
    pub emissive: Option<TextureSlot>,
    pub opacity: Option<TextureSlot>,
    pub ambient_occlusion: Option<TextureSlot>,
}

#[derive(Debug, Clone)]
pub struct TextureSlot {
    pub export_index: Option<usize>,
    pub uv_index: u32,
}

pub fn read_material(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    export_idx: usize,
) -> Result<MaterialAsset, UnrealError> {
    let export = header.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_material",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();

    let start_offset = export.serial_offset as u64;
    header.archive.seek(SeekFrom::Start(start_offset))?;

    let pr = PropertyReader::new(&header.names);

    let mut mat = MaterialAsset {
        name: export_name,
        base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
        metallic: 0.0,
        roughness: 0.5,
        specular: 0.5,
        emissive_color: Vec3::ZERO,
        opacity: 1.0,
        opacity_mask_clip_value: 0.333,
        blend_mode: 0,
        shading_model: 0,
        two_sided: false,
        textures: MaterialTextures::default(),
    };

    loop {
        match pr.read_tag(&mut header.archive)? {
            None => break,
            Some(tag) => {
                match tag.name.as_str() {
                    "BaseColor" => {
                        if tag.type_name == "LinearColor" {
                            mat.base_color = Vec4::new(
                                reader::read_f32(&mut header.archive)?,
                                reader::read_f32(&mut header.archive)?,
                                reader::read_f32(&mut header.archive)?,
                                reader::read_f32(&mut header.archive)?,
                            );
                        } else if tag.type_name == "Color" {
                            let packed = reader::read_u32(&mut header.archive)?;
                            let b = ((packed >> 0) & 0xFF) as f32 / 255.0;
                            let g = ((packed >> 8) & 0xFF) as f32 / 255.0;
                            let r = ((packed >> 16) & 0xFF) as f32 / 255.0;
                            let a = ((packed >> 24) & 0xFF) as f32 / 255.0;
                            mat.base_color = Vec4::new(r, g, b, a);
                        } else {
                            let next = tag.raw.next_offset;
                            let pos = match header.archive.stream_position() {
                                Ok(p) => p,
                                Err(_) => 0,
                            };
                            if next > pos {
                                header.archive.seek(SeekFrom::Start(next))?;
                            }
                        }
                    }
                    "Metallic" | "Roughness" | "Specular" | "Opacity" | "OpacityMaskClipValue" => {
                        let val = reader::read_f32(&mut header.archive)?;
                        match tag.name.as_str() {
                            "Metallic" => mat.metallic = val,
                            "Roughness" => mat.roughness = val,
                            "Specular" => mat.specular = val,
                            "Opacity" => mat.opacity = val,
                            "OpacityMaskClipValue" => mat.opacity_mask_clip_value = val,
                            _ => {}
                        }
                    }
                    "EmissiveColor" => {
                        if tag.type_name == "LinearColor" {
                            mat.emissive_color = Vec3::new(
                                reader::read_f32(&mut header.archive)?,
                                reader::read_f32(&mut header.archive)?,
                                reader::read_f32(&mut header.archive)?,
                            );
                            let _a = reader::read_f32(&mut header.archive)?;
                        } else {
                            let next = tag.raw.next_offset;
                            let pos = match header.archive.stream_position() {
                                Ok(p) => p,
                                Err(_) => 0,
                            };
                            if next > pos {
                                header.archive.seek(SeekFrom::Start(next))?;
                            }
                        }
                    }
                    "BlendMode" => {
                        mat.blend_mode = reader::read_u8(&mut header.archive)? as u32;
                    }
                    "ShadingModel" => {
                        mat.shading_model = reader::read_u8(&mut header.archive)? as u32;
                    }
                    "TwoSided" => {
                        mat.two_sided = reader::read_u32(&mut header.archive)? != 0;
                    }
                    "ParameterValues" | "ScalarParameterValues"
                    | "VectorParameterValues" | "TextureParameterValues" => {
                        parse_parameter_values(&pr, &mut header.archive, &tag, &mut mat)?;
                    }
                    "Parent" => {}
                    _ => {
                        let next = tag.raw.next_offset;
                        let pos = match header.archive.stream_position() {
                            Ok(p) => p,
                            Err(_) => 0,
                        };
                        if next > pos {
                            header.archive.seek(SeekFrom::Start(next))?;
                        }
                    }
                }
            }
        }
    }

    Ok(mat)
}

fn parse_parameter_values(
    _pr: &PropertyReader,
    ar: &mut uasset::Archive<std::io::Cursor<&[u8]>>,
    tag: &crate::uobject::property::PropertyTagInfo,
    _mat: &mut MaterialAsset,
) -> Result<(), UnrealError> {
    match tag.name.as_str() {
        "TextureParameterValues" => {
            let count = reader::read_i32(ar)? as usize;
            for _ in 0..count {
                let _param_name = reader::read_fname(ar)?;
                let _texture_ref = reader::read_package_index(ar)?;
            }
        }
        "ScalarParameterValues" => {
            let count = reader::read_i32(ar)? as usize;
            for _ in 0..count {
                let _param_name = reader::read_fname(ar)?;
                let _value = reader::read_f32(ar)?;
            }
        }
        "VectorParameterValues" => {
            let count = reader::read_i32(ar)? as usize;
            for _ in 0..count {
                let _param_name = reader::read_fname(ar)?;
                let _r = reader::read_f32(ar)?;
                let _g = reader::read_f32(ar)?;
                let _b = reader::read_f32(ar)?;
                let _a = reader::read_f32(ar)?;
            }
        }
        _ => {
            let count = reader::read_i32(ar)? as usize;
            for _ in 0..count {
                let _ = reader::read_i32(ar)?;
            }
        }
    }

    Ok(())
}
