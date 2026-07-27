use std::io::SeekFrom;

use glam::{Vec3, Vec4};

use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::uobject::property::PropertyReader;
use crate::UPackage;

/// Parsed material data from a UMaterial or UMaterialInstanceConstant.
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
    /// Index into the package's export table for the referenced texture.
    pub export_index: Option<usize>,
    /// UV channel index.
    pub uv_index: u32,
}

/// Read a UMaterialInstanceConstant or UMaterial from a package export.
pub fn read_material(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<MaterialAsset, UnrealError> {
    let export = pkg.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_material",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = pkg.resolve_name(export.object_name);
    let _class_name = if export.class_index.is_export() {
        let class_idx = (export.class_index.0 - 1) as usize;
        pkg.resolve_name(
            pkg.exports.get(class_idx).map(|e| e.object_name)
                .unwrap_or(crate::types::FName::new(0, 0)),
        )
    } else {
        String::new()
    };

    let start_offset = if pkg.version.is_ue5() {
        export.serial_offset as u64
    } else {
        export.serial_offset as u64
    };

    reader.seek(SeekFrom::Start(start_offset))?;

    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

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
        match pr.read_tag()? {
            None => break,
            Some(tag) => {
                let ar = pr.archive();
                match tag.name.as_str() {
                    "BaseColor" => {
                        // FColor or FLinearColor property
                        if tag.type_name == "LinearColor" {
                            mat.base_color = Vec4::new(
                                ar.read_f32()?,
                                ar.read_f32()?,
                                ar.read_f32()?,
                                ar.read_f32()?,
                            );
                        } else if tag.type_name == "Color" {
                            let packed = ar.read_u32()?;
                            let b = ((packed >> 0) & 0xFF) as f32 / 255.0;
                            let g = ((packed >> 8) & 0xFF) as f32 / 255.0;
                            let r = ((packed >> 16) & 0xFF) as f32 / 255.0;
                            let a = ((packed >> 24) & 0xFF) as f32 / 255.0;
                            mat.base_color = Vec4::new(r, g, b, a);
                        } else {
                            let data_start = ar.pos;
                            if tag.raw.next_offset > data_start {
                                ar.seek_to(tag.raw.next_offset)?;
                            }
                        }
                    }
                    "Metallic" | "Roughness" | "Specular" | "Opacity" | "OpacityMaskClipValue" => {
                        let val = ar.read_f32()?;
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
                                ar.read_f32()?,
                                ar.read_f32()?,
                                ar.read_f32()?,
                            );
                            let _a = ar.read_f32()?; // alpha
                        } else {
                            let data_start = ar.pos;
                            if tag.raw.next_offset > data_start {
                                ar.seek_to(tag.raw.next_offset)?;
                            }
                        }
                    }
                    "BlendMode" => {
                        // ByteProperty (u8)
                        mat.blend_mode = ar.read_u8()? as u32;
                    }
                    "ShadingModel" => {
                        // ByteProperty (u8)
                        mat.shading_model = ar.read_u8()? as u32;
                    }
                    "TwoSided" => {
                        // BoolProperty (u32)
                        mat.two_sided = ar.read_u32()? != 0;
                    }
                    // Material instance parameter values
                    "ParameterValues" | "ScalarParameterValues"
                    | "VectorParameterValues" | "TextureParameterValues" => {
                        // These are arrays of parameter structs in MIC.
                        // For TextureParameterValues, each entry has a name + texture reference.
                        parse_parameter_values(&mut pr, tag, &mut mat)?;
                    }
                    // Parent material (for MIC)
                    "Parent" => {
                        // ObjectProperty — reference to parent material
                        // We skip this for now; in a full impl we'd recurse
                    }
                    _ => {
                        let data_start = ar.pos;
                        if tag.raw.next_offset > data_start {
                            ar.seek_to(tag.raw.next_offset)?;
                        }
                    }
                }
            }
        }
    }

    Ok(mat)
}

/// Parse texture/material parameter value arrays from MIC/MPC.
fn parse_parameter_values(
    pr: &mut PropertyReader,
    tag: crate::uobject::property::PropertyTagInfo,
    _mat: &mut MaterialAsset,
) -> Result<(), UnrealError> {
    let ar = pr.archive();

    match tag.name.as_str() {
        "TextureParameterValues" => {
            // FTextureParameterValue array
            // Each entry: ParameterName (FName), ParameterValue (Texture2D ObjectPtr)
            let count = ar.read_serial_size()? as usize;
            for _ in 0..count {
                let _param_name = ar.read_fname()?;
                let _texture_ref = ar.read_package_index()?; // FPackageIndex to the texture
                // TODO: map param_name to the correct texture slot and resolve the export index
            }
        }
        "ScalarParameterValues" => {
            // Each entry: ParameterName (FName), ParameterValue (f32)
            let count = ar.read_serial_size()? as usize;
            for _ in 0..count {
                let _param_name = ar.read_fname()?;
                let _value = ar.read_f32()?;
                // TODO: map param_name to material properties (e.g., "Roughness" -> mat.roughness)
            }
        }
        "VectorParameterValues" => {
            // Each entry: ParameterName (FName), ParameterValue (FLinearColor = 4xf32)
            let count = ar.read_serial_size()? as usize;
            for _ in 0..count {
                let _param_name = ar.read_fname()?;
                let _r = ar.read_f32()?;
                let _g = ar.read_f32()?;
                let _b = ar.read_f32()?;
                let _a = ar.read_f32()?;
            }
        }
        _ => {
            // Skip unknown array
            let count = ar.read_serial_size()? as usize;
            for _ in 0..count {
                let _ = ar.read_i32()?;
            }
        }
    }

    Ok(())
}
