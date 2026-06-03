use std::ffi::OsString;
use std::fs;
use std::io::Write;

use solid_gltf::GltfSaver;
use solid_rs::prelude::*;

use crate::bridge::{run_blender, TempDirGuard};
use crate::BLEND_FORMAT;

pub struct BlendSaver;

impl Saver for BlendSaver {
    fn format_info(&self) -> &'static FormatInfo {
        &BLEND_FORMAT
    }

    fn save(&self, scene: &Scene, writer: &mut dyn Write, _options: &SaveOptions) -> Result<()> {
        let temp = TempDirGuard::new("solid-blend-save")?;
        let glb_path = temp.path().join("input.glb");
        let blend_path = temp.path().join("output.blend");

        let mut glb_bytes = Vec::new();
        GltfSaver.save_glb(scene, &mut glb_bytes)?;
        fs::write(&glb_path, &glb_bytes).map_err(SolidError::Io)?;

        import_glb_and_save_blend(&glb_path, &blend_path)?;

        let out = fs::read(&blend_path).map_err(|e| {
            SolidError::format(
                "blend",
                format!(
                    "Failed reading Blender output at {}: {e}",
                    blend_path.display()
                ),
            )
        })?;
        writer.write_all(&out).map_err(SolidError::Io)
    }
}

fn import_glb_and_save_blend(
    glb_path: &std::path::Path,
    blend_path: &std::path::Path,
) -> Result<()> {
    let script = "import bpy,sys; argv=sys.argv[sys.argv.index('--')+1:]; src=argv[0]; dst=argv[1]; bpy.ops.wm.read_factory_settings(use_empty=True); bpy.ops.import_scene.gltf(filepath=src); bpy.ops.wm.save_as_mainfile(filepath=dst, check_existing=False, compress=False)";
    let args = vec![
        OsString::from("--background"),
        OsString::from("--factory-startup"),
        OsString::from("--disable-autoexec"),
        OsString::from("--python-expr"),
        OsString::from(script),
        OsString::from("--"),
        glb_path.as_os_str().to_owned(),
        blend_path.as_os_str().to_owned(),
    ];
    run_blender(&args)
}
