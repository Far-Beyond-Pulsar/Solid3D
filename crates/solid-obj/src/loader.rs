//! `ObjLoader` — loads Wavefront OBJ files into a `solid_rs::Scene`.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use solid_rs::prelude::*;
use solid_rs::scene::Scene;
use solid_rs::{Result, SolidError};

use crate::{convert, OBJ_FORMAT};
use crate::parser::{parse_mtl, parse_obj};

/// Loader for Wavefront OBJ files (`.obj`).
///
/// MTL material libraries are resolved from `LoadOptions::base_dir` when
/// provided.  Without a `base_dir` the geometry is still loaded, but all
/// materials will be plain white.
pub struct ObjLoader;

impl Loader for ObjLoader {
    fn format_info(&self) -> &FormatInfo {
        &OBJ_FORMAT
    }

    fn load(
        &self,
        reader: &mut dyn ReadSeek,
        options: &LoadOptions,
    ) -> Result<Scene> {
        let mut src = String::new();
        reader.read_to_string(&mut src).map_err(SolidError::Io)?;

        let obj = parse_obj(&src);

        // Try to load MTL files from base_dir
        let mtl_data = if let Some(base) = &options.base_dir {
            load_mtls(&obj.mtllibs, base)
        } else {
            None
        };

        Ok(convert::obj_to_scene(&obj, mtl_data.as_ref()))
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        // OBJ is plain text; look for `v `, `vn `, `f ` near the start
        let mut buf = [0u8; 512];
        let n = reader.read(&mut buf).unwrap_or(0);
        let s = std::str::from_utf8(&buf[..n]).unwrap_or("");
        let score = s.lines().take(20).filter(|l| {
            let l = l.trim_start();
            l.starts_with("v ") || l.starts_with("vn ") || l.starts_with("vt ") || l.starts_with("f ")
        }).count();
        (score as f32 / 5.0).min(0.9)
    }
}

/// Attempt to parse MTL files from `base_dir`.  Returns `None` if none
/// could be found/read (non-fatal; geometry loading continues).
fn load_mtls(mtllibs: &[String], base_dir: &Path) -> Option<crate::parser::MtlData> {
    let mut combined = crate::parser::MtlData::default();
    let mut any = false;
    for lib in mtllibs {
        let path = base_dir.join(lib);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let mtl = parse_mtl(&content);
            combined.materials.extend(mtl.materials);
            any = true;
        }
    }
    if any { Some(combined) } else { None }
}
