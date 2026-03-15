# solid-obj

**Wavefront OBJ/MTL 3D format support for the [SolidRS](https://github.com/Far-Beyond-Pulsar/solid-rs) ecosystem.**

`solid-obj` is a format crate in the SolidRS family — the relationship mirrors `serde` / `serde_json`: `solid-rs` provides the shared scene types and traits while `solid-obj` plugs in Wavefront OBJ read/write support.

---

## Features

| Capability | Load | Save |
|---|---|---|
| Vertex positions (`v`) | ✅ | ✅ |
| Normals (`vn`) | ✅ | ✅ |
| UV coordinates (`vt`) | ✅ | ✅ |
| Objects & groups (`o`, `g`) | ✅ | ✅ |
| Material references (`usemtl`) | ✅ | ✅ |
| MTL library loading (`mtllib`) | ✅ | ✅ |
| Negative / relative indices | ✅ | — |
| N-gon fan triangulation | ✅ | — |
| Per-group vertex deduplication | ✅ | — |
| Diffuse / emissive / alpha | ✅ | ✅ |
| Texture maps (`map_Kd`, `map_bump`, …) | ✅ | ✅ |
| Embedded MTL in saved OBJ | — | ✅ |
| Separate MTL writer | — | ✅ |
| Skinning / animations | ❌ | ❌ |

---

## Installation

```toml
[dependencies]
solid-rs  = "0.1"
solid-obj = "0.1"
```

---

## Quick start

```rust
use solid_rs::registry::Registry;
use solid_obj::{ObjLoader, ObjSaver};

fn main() {
    let mut registry = Registry::new();
    registry.register_loader(ObjLoader);
    registry.register_saver(ObjSaver);

    let scene = registry.load_file("mesh.obj").unwrap();
    println!("meshes:    {}", scene.meshes.len());
    println!("materials: {}", scene.materials.len());

    registry.save_file(&scene, "out.obj").unwrap();
}
```

---

## Loading with MTL materials

MTL material libraries are resolved from disk when you provide a
`base_dir` in `LoadOptions`.  Without one, geometry loads fine but
all materials are plain white.

```rust
use solid_rs::prelude::*;
use solid_obj::ObjLoader;
use std::path::PathBuf;

let loader = ObjLoader;
let opts = LoadOptions {
    base_dir: Some(PathBuf::from("assets/")),
    ..Default::default()
};

let mut file = std::fs::File::open("assets/model.obj").unwrap();
let scene = loader.load(&mut file, &opts).unwrap();
```

---

## Saving — OBJ + MTL

By default `ObjSaver` writes the MTL content as an embedded comment
block at the end of the `.obj` stream (delimited by `# MTL BEGIN` /
`# MTL END`).  To write a separate `.mtl` file, use `ObjSaver::save_mtl`:

```rust
use solid_obj::ObjSaver;
use solid_rs::prelude::*;

// Write geometry to one file, materials to another
let mut obj_file = std::fs::File::create("out.obj").unwrap();
let mut mtl_file = std::fs::File::create("out.mtl").unwrap();

let saver = ObjSaver;
saver.save(&scene, &mut obj_file, &SaveOptions::default()).unwrap();
ObjSaver::save_mtl(&scene, &mut mtl_file).unwrap();
```

---

## Format support

### Face syntax

All standard face vertex reference forms are parsed:

| Syntax | Meaning |
|---|---|
| `f v` | position only |
| `f v/vt` | position + UV |
| `f v//vn` | position + normal |
| `f v/vt/vn` | position + UV + normal |

Negative indices (e.g. `f -1 -2 -3`) are resolved relative to the
end of the current vertex pool.

Quads and n-gons are **fan-triangulated**: a polygon
`(v0, v1, v2, v3, v4)` becomes triangles
`(v0,v1,v2)`, `(v0,v2,v3)`, `(v0,v3,v4)`.

### MTL properties mapped to PBR

| MTL key | PBR field |
|---|---|
| `Kd r g b` | `base_color_factor` (RGB) |
| `d` / `Tr` | `base_color_factor` alpha |
| `Ke r g b` | `emissive_factor` |
| `Ns` | `roughness_factor` (approximated: `sqrt(1 - Ns/1000)`) |
| `map_Kd` | `base_color_texture` |
| `map_Ks` / `map_Pr` | `metallic_roughness_texture` |
| `map_bump` / `bump` / `norm` | `normal_texture` |

### Multi-material meshes

When an OBJ group references multiple materials via `usemtl`, each
contiguous block of faces sharing the same material becomes a separate
`Primitive` on the mesh.  This matches the `solid_rs::scene::Mesh`
model where `primitives` carry per-draw-call material indices.

---

## Transcoding example

Because all SolidRS format crates share the same `solid_rs::Scene` IR,
transcoding between formats is one-liner once both are registered:

```rust
use solid_rs::registry::Registry;
use solid_fbx::{FbxLoader, FbxSaver};
use solid_obj::{ObjLoader, ObjSaver};

let mut reg = Registry::new();
reg.register_loader(FbxLoader);
reg.register_loader(ObjLoader);
reg.register_saver(FbxSaver);
reg.register_saver(ObjSaver);

let scene = reg.load_file("model.fbx").unwrap(); // reads binary/ASCII FBX
reg.save_file(&scene, "model.obj").unwrap();      // writes Wavefront OBJ
```

---

## Crate layout

```
solid-obj/
├── src/
│   ├── lib.rs       — public API, OBJ_FORMAT static
│   ├── parser.rs    — OBJ + MTL text parsers → ObjData / MtlData IR
│   ├── convert.rs   — ObjData + MtlData → solid_rs::Scene
│   ├── loader.rs    — ObjLoader (implements solid_rs::traits::Loader)
│   └── saver.rs     — ObjSaver (implements solid_rs::traits::Saver)
```

---

## License

MIT — see [LICENSE](../../LICENSE).
