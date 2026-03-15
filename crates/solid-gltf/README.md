# solid-gltf

glTF 2.0 / GLB loader and saver for the [SolidRS](https://github.com/Far-Beyond-Pulsar/solid-rs) ecosystem.

## Feature Matrix

| Feature | Load | Save |
|:--------|:----:|:----:|
| glTF 2.0 JSON (`.gltf`) | ✅ | ✅ |
| GLB binary container (`.glb`) | ✅ | ✅ |
| Embedded base64 buffers | ✅ | ✅ |
| External buffer files | ✅ | — |
| Meshes (positions, normals, tangents) | ✅ | ✅ |
| UV channels (TEXCOORD_0–7) | ✅ | ✅ (ch 0) |
| Vertex colours (COLOR_0) | ✅ | ✅ |
| Skinning (JOINTS_0 / WEIGHTS_0) | ✅ | — |
| PBR metallic-roughness materials | ✅ | ✅ |
| Textures & images (URI / embedded) | ✅ | ✅ |
| Cameras (perspective & orthographic) | ✅ | ✅ |
| Scene hierarchy (nodes, TRS, matrix) | ✅ | ✅ |
| Animations | — | — |
| Skins / skeletons | — | — |
| Extensions (KHR_*) | — | — |

## Installation

```toml
[dependencies]
solid-rs   = "0.1"
solid-gltf = "0.1"
```

## Quick Start

### Load a glTF file

```rust
use solid_gltf::GltfLoader;
use solid_rs::traits::{Loader, LoadOptions};
use std::fs::File;
use std::io::BufReader;

fn main() -> solid_rs::Result<()> {
    let file = File::open("model.gltf")?;
    let mut reader = BufReader::new(file);
    let options = LoadOptions {
        base_dir: Some("assets/".into()),
        ..Default::default()
    };
    let scene = GltfLoader.load(&mut reader, &options)?;
    println!("Loaded {} meshes, {} materials", scene.meshes.len(), scene.materials.len());
    Ok(())
}
```

### Load a GLB file

```rust
use solid_gltf::GltfLoader;
use solid_rs::traits::{Loader, LoadOptions};
use std::fs::File;
use std::io::BufReader;

fn main() -> solid_rs::Result<()> {
    let file = File::open("model.glb")?;
    let mut reader = BufReader::new(file);
    let scene = GltfLoader.load(&mut reader, &LoadOptions::default())?;
    println!("Loaded {} nodes", scene.nodes.len());
    Ok(())
}
```

### Save as glTF JSON (buffer embedded as base64)

```rust
use solid_gltf::GltfSaver;
use solid_rs::traits::{Saver, SaveOptions};
use std::fs::File;
use std::io::BufWriter;

fn save_gltf(scene: &solid_rs::Scene) -> solid_rs::Result<()> {
    let file = File::create("output.gltf")?;
    let mut writer = BufWriter::new(file);
    let options = SaveOptions { pretty_print: true, ..Default::default() };
    GltfSaver.save(scene, &mut writer, &options)
}
```

### Save as GLB binary

```rust
use solid_gltf::GltfSaver;
use std::fs::File;
use std::io::BufWriter;

fn save_glb(scene: &solid_rs::Scene) -> solid_rs::Result<()> {
    let file = File::create("output.glb")?;
    let mut writer = BufWriter::new(file);
    GltfSaver.save_glb(scene, &mut writer)
}
```

## Format Internals

### GLB Container Layout

```
Byte offset  Description
───────────  ──────────────────────────────────
0            Magic: 0x46546C67 ("glTF")
4            Version: 2
8            Total file length (u32 LE)
12           Chunk 0 length (JSON, u32 LE)
16           Chunk 0 type: 0x4E4F534A ("JSON")
20           JSON payload (padded with 0x20)
20+json_len  Chunk 1 length (BIN, u32 LE)
+4           Chunk 1 type: 0x004E4942 ("BIN\0")
+8           Binary payload (padded with 0x00)
```

### Accessors and Buffer Layout

Each vertex attribute and index array is stored as a contiguous region in
the binary buffer, referenced by a **bufferView** and **accessor**:

- **bufferView**: `{ buffer, byteOffset, byteLength, byteStride?, target }`
- **accessor**: `{ bufferView, byteOffset, componentType, count, type }`

Component types used by this crate:

| Code | Type | Size |
|:----:|:----:|:----:|
| 5126 | FLOAT | 4 bytes |
| 5125 | UNSIGNED_INT | 4 bytes |
| 5123 | UNSIGNED_SHORT | 2 bytes |
| 5121 | UNSIGNED_BYTE | 1 byte |

## Material Mapping

| glTF PBR field | solid-rs `Material` field |
|:---------------|:--------------------------|
| `pbrMetallicRoughness.baseColorFactor` | `base_color_factor: Vec4` |
| `pbrMetallicRoughness.baseColorTexture` | `base_color_texture: Option<TextureRef>` |
| `pbrMetallicRoughness.metallicFactor` | `metallic_factor: f32` |
| `pbrMetallicRoughness.roughnessFactor` | `roughness_factor: f32` |
| `pbrMetallicRoughness.metallicRoughnessTexture` | `metallic_roughness_texture: Option<TextureRef>` |
| `normalTexture` | `normal_texture: Option<TextureRef>` |
| `normalTexture.scale` | `normal_scale: f32` |
| `occlusionTexture` | `occlusion_texture: Option<TextureRef>` |
| `occlusionTexture.strength` | `occlusion_strength: f32` |
| `emissiveFactor` | `emissive_factor: Vec3` |
| `emissiveTexture` | `emissive_texture: Option<TextureRef>` |
| `alphaMode` | `alpha_mode: AlphaMode` |
| `alphaCutoff` | `alpha_cutoff: f32` |
| `doubleSided` | `double_sided: bool` |

## Crate Layout

```
solid-gltf/src/
├── lib.rs        — Public API: GltfLoader, GltfSaver, GLTF_FORMAT
├── document.rs   — serde structs for the glTF JSON DOM
├── buffer.rs     — Buffer resolution (URI / base64 / GLB) and accessor reads
├── convert.rs    — Bidirectional conversion: GltfRoot ↔ solid_rs::Scene
├── loader.rs     — Loader trait impl (glTF JSON + GLB parsing)
└── saver.rs      — Saver trait impl (glTF JSON + GLB writing)
```

## License

MIT — see the workspace [`LICENSE`](../../LICENSE) file.
