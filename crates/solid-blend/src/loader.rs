use std::ffi::OsString;
use std::fs;
use std::io::{Cursor, Read};

use solid_gltf::GltfLoader;
use solid_rs::prelude::*;

use crate::bridge::{run_blender, TempDirGuard};
use crate::BLEND_FORMAT;

pub struct BlendLoader;

impl Loader for BlendLoader {
    fn format_info(&self) -> &'static FormatInfo {
        &BLEND_FORMAT
    }

    fn detect(&self, reader: &mut dyn Read) -> f32 {
        let mut buf = [0u8; 12];
        let n = reader.read(&mut buf).unwrap_or(0);
        let header = &buf[..n];
        if header.starts_with(b"BLENDER") {
            return 0.98;
        }
        0.0
    }

    fn load(&self, reader: &mut dyn ReadSeek, options: &LoadOptions) -> Result<Scene> {
        let mut blend_bytes = Vec::new();
        reader
            .read_to_end(&mut blend_bytes)
            .map_err(SolidError::Io)?;
        if blend_bytes.len() < 7 || !blend_bytes.starts_with(b"BLENDER") {
            return Err(SolidError::parse(
                "Blender file header missing (expected 'BLENDER')",
            ));
        }

        let temp = TempDirGuard::new("solid-blend-load")?;
        let blend_path = temp.path().join("input.blend");
        let glb_path = temp.path().join("output.glb");

        fs::write(&blend_path, &blend_bytes).map_err(SolidError::Io)?;
        export_blend_to_glb(&blend_path, &glb_path)?;

        let glb_bytes = fs::read(&glb_path).map_err(|e| {
            SolidError::format(
                "blend",
                format!(
                    "Failed reading converted GLB at {}: {e}",
                    glb_path.display()
                ),
            )
        })?;

        GltfLoader.load(&mut Cursor::new(glb_bytes), options)
    }
}

fn export_blend_to_glb(blend_path: &std::path::Path, glb_path: &std::path::Path) -> Result<()> {
    let script = "import bpy,sys; argv=sys.argv[sys.argv.index('--')+1:]; out=argv[0]; bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_apply=True)";
    let args = vec![
        OsString::from("--background"),
        OsString::from("--factory-startup"),
        OsString::from("--disable-autoexec"),
        blend_path.as_os_str().to_owned(),
        OsString::from("--python-expr"),
        OsString::from(script),
        OsString::from("--"),
        glb_path.as_os_str().to_owned(),
    ];
    run_blender(&args)
}
