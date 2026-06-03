# solid-blend

Blender `.blend` format support for `solid-rs` via Blender CLI conversion.

## How it works

- Load: `.blend` → Blender headless export to temporary `.glb` → `solid-gltf` loader
- Save: scene → `solid-gltf` GLB → Blender headless import/export to `.blend`

## Requirements

- Blender installed and accessible as `blender`, or set `BLENDER_BIN`.
- Optional timeout override: `BLENDER_TIMEOUT_SECS`.

## Usage

```rust
use solid_rs::registry::Registry;
use solid_blend::{BlendLoader, BlendSaver};

let mut reg = Registry::new();
reg.register_loader(BlendLoader);
reg.register_saver(BlendSaver);
```
