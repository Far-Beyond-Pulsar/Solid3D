# solid-fbx

**Autodesk FBX 3D format support for the [SolidRS](https://github.com/Far-Beyond-Pulsar/solid-rs) ecosystem.**

`solid-fbx` is a format crate in the SolidRS family — the relationship mirrors `serde` / `serde_json`: `solid-rs` provides the shared scene types and traits while `solid-fbx` plugs in FBX read/write support.

---

## Features

| Capability | Load | Save |
|---|---|---|
| Binary FBX (v7.2 – v7.7) | ✅ | — |
| ASCII FBX (v7.4) | ✅ | ✅ |
| Geometry — positions, normals, UVs | ✅ | ✅ |
| N-gon fan triangulation | ✅ | — |
| Node hierarchy + local transforms | ✅ | ✅ |
| Materials — diffuse / emissive | ✅ | ✅ |
| Texture filename references | ✅ | ✅ |
| Zlib-compressed array properties | ✅ | — |
| Euler-degree ↔ quaternion conversion | ✅ | ✅ |
| Skinning / skeletal animation | ❌ | ❌ |

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
solid-rs  = "0.1"
solid-fbx = "0.1"
```

---

## Quick start

```rust
use solid_rs::registry::Registry;
use solid_fbx::{FbxLoader, FbxSaver};

fn main() {
    let mut registry = Registry::new();
    registry.register_loader(FbxLoader);
    registry.register_saver(FbxSaver);

    // Load an FBX file (binary or ASCII detected automatically)
    let scene = registry.load_file("character.fbx").unwrap();
    println!("meshes:    {}", scene.meshes.len());
    println!("materials: {}", scene.materials.len());
    println!("nodes:     {}", scene.nodes.len());

    // Save back as ASCII FBX 7.4
    registry.save_file(&scene, "out.fbx").unwrap();
}
```

---

## Loading directly with options

```rust
use solid_rs::prelude::*;
use solid_fbx::FbxLoader;

let loader = FbxLoader;
let mut file = std::fs::File::open("model.fbx").unwrap();
let scene = loader.load(&mut file, &LoadOptions::default()).unwrap();
```

---

## Registering in a shared registry

```rust
use solid_rs::registry::Registry;
use solid_fbx::{FbxLoader, FbxSaver};
use solid_obj::{ObjLoader, ObjSaver};   // another format crate

let mut reg = Registry::new();
reg.register_loader(FbxLoader);
reg.register_loader(ObjLoader);
reg.register_saver(FbxSaver);
reg.register_saver(ObjSaver);

// Registry auto-detects format from extension / magic bytes
let scene = reg.load_file("scene.fbx").unwrap();
reg.save_file(&scene, "scene.obj").unwrap(); // transcode!
```

---

## Format notes

### Binary FBX

Binary files begin with the 23-byte magic sequence:

```
Kaydara FBX Binary  \x00\x1a\x00
```

followed by a `u32` version number.  `solid-fbx` supports both the
32-bit offset format (version < 7500) and the 64-bit offset format
(version ≥ 7500 / MotionBuilder 2016+).

Array properties (type codes `f`, `d`, `i`, `l`, `b`) may be stored
compressed with zlib/deflate (encoding = 1); these are decompressed
transparently using the [`flate2`](https://crates.io/crates/flate2) crate.

### ASCII FBX

ASCII files begin with `; FBX` and use a human-readable node / property
syntax.  The saver always writes ASCII FBX 7.4.

### Transforms

FBX stores local transforms as three separate `Properties70` entries
(`LclTranslation`, `LclRotation`, `LclScaling`) where rotation angles
are in **degrees** and use XYZ Euler order.  On load these are converted
to a `solid_rs::geometry::Transform` (translation + quaternion + scale).
On save the quaternion is converted back to XYZ Euler degrees.

### Object connections

FBX wires objects together through a `Connections` section.  Two
connection types are supported:

- **OO** (object–object): geometry → model, material → model,
  model → parent model
- **OP** (object–property): texture → material channel
  (`DiffuseColor`, `NormalMap`)

---

## Crate layout

```
solid-fbx/
├── src/
│   ├── lib.rs        — public API, FBX_FORMAT static
│   ├── document.rs   — FBX DOM: FbxDocument / FbxNode / FbxProperty
│   ├── binary.rs     — binary FBX parser (magic detection, node reader)
│   ├── ascii.rs      — ASCII FBX tokeniser + recursive descent parser
│   ├── convert.rs    — FbxDocument → solid_rs::Scene conversion
│   ├── loader.rs     — FbxLoader (implements solid_rs::traits::Loader)
│   └── saver.rs      — FbxSaver (implements solid_rs::traits::Saver)
```

---

## License

MIT — see [LICENSE](../../LICENSE).
