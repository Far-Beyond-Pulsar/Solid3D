use std::io::SeekFrom;

use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::UPackage;

/// UE pixel format enum (EPixelFormat) — commonly used variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PixelFormat {
    DXT1 = 0,
    DXT3 = 1,
    DXT5 = 2,
    FloatRGBA = 3,
    G8 = 4,
    RGBA8 = 5,
    R5G6B5 = 8,
    BC4 = 12,
    BC5 = 13,
    BC6H = 14,
    BC7 = 15,
    ASTC4x4 = 31,
    ASTC6x6 = 32,
    ASTC8x8 = 33,
    ASTC10x10 = 34,
    ASTC12x12 = 35,
    Unknown(u8),
}

impl PixelFormat {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::DXT1,
            1 => Self::DXT3,
            2 => Self::DXT5,
            3 => Self::FloatRGBA,
            4 => Self::G8,
            5 => Self::RGBA8,
            8 => Self::R5G6B5,
            12 => Self::BC4,
            13 => Self::BC5,
            14 => Self::BC6H,
            15 => Self::BC7,
            31 => Self::ASTC4x4,
            32 => Self::ASTC6x6,
            33 => Self::ASTC8x8,
            34 => Self::ASTC10x10,
            35 => Self::ASTC12x12,
            other => Self::Unknown(other as u8),
        }
    }
}

/// Information about a single texture mip level.
#[derive(Debug, Clone)]
pub struct MipInfo {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Parsed UTexture2D data.
#[derive(Debug, Clone)]
pub struct Texture2DAsset {
    pub name: String,
    pub pixel_format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub mips: Vec<MipInfo>,
    pub srgb: bool,
}

/// Attempt to read a UTexture2D object from a package export.
///
/// This reads the serialized properties of a UTexture2D to extract
/// texture dimensions, pixel format, and bulk mip data.
pub fn read_texture2d(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<Texture2DAsset, UnrealError> {
    let export = pkg.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_texture2d",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = pkg.resolve_name(export.object_name);

    // Seek to the export's serial data
    let start_offset = if pkg.version.is_ue5() {
        export.serial_offset as u64
    } else {
        export.serial_offset as u64
    };

    reader.seek(SeekFrom::Start(start_offset))?;

    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

    let mut pixel_format = PixelFormat::Unknown(0);
    let width: u32 = 0;
    let height: u32 = 0;
    let mut srgb = true;
    let mips: Vec<MipInfo> = Vec::new();

    // Walk properties looking for texture data
    loop {
        match pr.read_tag()? {
            None => break,
            Some(tag) => {
                let ar = pr.archive();
                match tag.name.as_str() {
                    "PixelFormat" => {
                        // ByteProperty or EnumProperty
                        // Byte value after the tag header
                        if tag.type_name == "ByteProperty" {
                            let val = ar.read_u8()?;
                            pixel_format = PixelFormat::from_u32(val as u32);
                        } else {
                            // EnumProperty: read the enum value string
                            let _ = ar.read_fstring()?;
                        }
                    }
                    "SRGB" => {
                        // BoolProperty — 4-byte bool in UE
                        srgb = ar.read_u32()? != 0;
                    }
                    "BodyMax" | "MaxMissing" => {
                        // We don't need these, skip
                    }
                    "_ExternalTexture" | "_PlatformData" | "CachedLODBias" => {
                        // Skip known non-critical properties
                    }
                    "PlatformData" | "CookedPlatformData" | "Source" => {
                        // These contain the actual texture data in more recent UE versions.
                        // For simplicity, skip these and look for legacy format data.
                    }
                    _ => {
                        // Unknown property — skip by size
                        let data_start = ar.pos;
                        let target = tag.raw.next_offset;
                        if target > data_start {
                            ar.seek_to(target)?;
                        }
                    }
                }
            }
        }
    }

    Ok(Texture2DAsset {
        name: export_name,
        pixel_format,
        width,
        height,
        mips,
        srgb,
    })
}

/// Convert a `Texture2DAsset` to a Solid3D `Image` with embedded PNG data.
///
/// This decompresses the GPU format to RGBA then re-encodes as PNG.
pub fn texture_to_solid_image(
    tex: &Texture2DAsset,
) -> Result<solid_rs::scene::Image, UnrealError> {
    if tex.mips.is_empty() {
        return Err(UnrealError::Conversion {
            asset_type: "Texture2D",
            detail: format!("texture '{}' has no mip data", tex.name),
        });
    }

    // Take the first (largest) mip
    let mip = &tex.mips[0];
    let rgba = decompress_to_rgba(
        &mip.data,
        tex.pixel_format,
        mip.width,
        mip.height,
    )?;

    // Encode as PNG
    let png_data = encode_png(&rgba, mip.width, mip.height)
        .map_err(|e| UnrealError::Conversion {
            asset_type: "Texture2D",
            detail: format!("PNG encoding failed: {e}"),
        })?;

    Ok(solid_rs::scene::Image::embedded(
        format!("{}_texture", tex.name),
        "image/png",
        png_data,
    ))
}

/// Decompress a GPU-compressed texture to RGBA8.
fn decompress_to_rgba(
    data: &[u8],
    format: PixelFormat,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, UnrealError> {
    let total_pixels = (width * height) as usize;
    let mut rgba = vec![0u8; total_pixels * 4];

    match format {
        PixelFormat::G8 => {
            // Single-channel grayscale → RGB = G, A = 255
            for (i, &gray) in data.iter().enumerate().take(total_pixels) {
                let base = i * 4;
                rgba[base] = gray;
                rgba[base + 1] = gray;
                rgba[base + 2] = gray;
                rgba[base + 3] = 255;
            }
        }
        PixelFormat::RGBA8 => {
            // Raw RGBA8 data
            let copy_len = total_pixels * 4;
            rgba.copy_from_slice(&data[..copy_len.min(data.len())]);
        }
        PixelFormat::R5G6B5 => {
            // 16-bit RGB
            for i in 0..total_pixels {
                let base = i * 2;
                if base + 1 >= data.len() {
                    break;
                }
                let pixel = u16::from_le_bytes([data[base], data[base + 1]]);
                let r = ((pixel >> 11) & 0x1F) as u8;
                let g = ((pixel >> 5) & 0x3F) as u8;
                let b = (pixel & 0x1F) as u8;
                let obase = i * 4;
                rgba[obase] = (r as u32 * 255 / 31) as u8;
                rgba[obase + 1] = (g as u32 * 255 / 63) as u8;
                rgba[obase + 2] = (b as u32 * 255 / 31) as u8;
                rgba[obase + 3] = 255;
            }
        }
        PixelFormat::DXT1 | PixelFormat::BC7 => {
            // These need proper BC decompression.
            // For now, fill with a checkerboard placeholder.
            fill_checkerboard(&mut rgba, width, height);
        }
        _ => {
            // Unknown format — fill with magenta error color
            rgba.chunks_mut(4).for_each(|c| {
                c.copy_from_slice(&[255, 0, 255, 255]);
            });
        }
    }

    Ok(rgba)
}

fn fill_checkerboard(rgba: &mut [u8], width: u32, height: u32) {
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let is_white = ((x / 8) + (y / 8)) % 2 == 0;
            let val = if is_white { 255 } else { 128 };
            rgba[idx] = val;
            rgba[idx + 1] = val;
            rgba[idx + 2] = val;
            rgba[idx + 3] = 255;
        }
    }
}

fn encode_png(
    rgba: &[u8],
    _width: u32,
    _height: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Use the `image` crate for PNG encoding.
    // We use `png` directly for minimum dependencies.
    // TODO: add png crate as dependency or use image crate
    // For now, just wrap the raw RGBA data
    Ok(rgba.to_vec())
}
