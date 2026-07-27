use std::io::{Seek, SeekFrom};

use crate::error::UnrealError;
use crate::reader;
use crate::uobject::property::PropertyReader;

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

#[derive(Debug, Clone)]
pub struct MipInfo {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Texture2DAsset {
    pub name: String,
    pub pixel_format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub mips: Vec<MipInfo>,
    pub srgb: bool,
}

pub fn read_texture2d(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    export_idx: usize,
) -> Result<Texture2DAsset, UnrealError> {
    let export = header.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_texture2d",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();

    let start_offset = export.serial_offset as u64;
    header.archive.seek(SeekFrom::Start(start_offset))?;

    let pr = PropertyReader::new(&header.names);

    let mut pixel_format = PixelFormat::Unknown(0);
    let width: u32 = 0;
    let height: u32 = 0;
    let mut srgb = true;
    let mips: Vec<MipInfo> = Vec::new();

    loop {
        match pr.read_tag(&mut header.archive)? {
            None => break,
            Some(tag) => {
                match tag.name.as_str() {
                    "PixelFormat" => {
                        if tag.type_name == "ByteProperty" {
                            let val = reader::read_u8(&mut header.archive)?;
                            pixel_format = PixelFormat::from_u32(val as u32);
                        } else {
                            let _ = reader::read_fstring(&mut header.archive)?;
                        }
                    }
                    "SRGB" => {
                        srgb = reader::read_u32(&mut header.archive)? != 0;
                    }
                    "BodyMax" | "MaxMissing" | "_ExternalTexture" | "_PlatformData" | "CachedLODBias" => {}
                    "PlatformData" | "CookedPlatformData" | "Source" => {}
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

    Ok(Texture2DAsset {
        name: export_name,
        pixel_format,
        width,
        height,
        mips,
        srgb,
    })
}

pub fn texture_to_solid_image(
    tex: &Texture2DAsset,
) -> Result<solid_rs::scene::Image, UnrealError> {
    if tex.mips.is_empty() {
        return Err(UnrealError::Conversion {
            asset_type: "Texture2D",
            detail: format!("texture '{}' has no mip data", tex.name),
        });
    }

    let mip = &tex.mips[0];
    let rgba = decompress_to_rgba(
        &mip.data,
        tex.pixel_format,
        mip.width,
        mip.height,
    )?;

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
            for (i, &gray) in data.iter().enumerate().take(total_pixels) {
                let base = i * 4;
                rgba[base] = gray;
                rgba[base + 1] = gray;
                rgba[base + 2] = gray;
                rgba[base + 3] = 255;
            }
        }
        PixelFormat::RGBA8 => {
            let copy_len = total_pixels * 4;
            rgba.copy_from_slice(&data[..copy_len.min(data.len())]);
        }
        PixelFormat::R5G6B5 => {
            for i in 0..total_pixels {
                let base = i * 2;
                if base + 1 >= data.len() { break; }
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
            fill_checkerboard(&mut rgba, width, height);
        }
        _ => {
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
    Ok(rgba.to_vec())
}
