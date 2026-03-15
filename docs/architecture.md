# Architecture

SolidRS is modelled directly on the `serde` / `serde_json` split:

| serde ecosystem | SolidRS ecosystem |
|:--------------- |:----------------- |
| `serde`         | `solid-rs`        |
| `serde_json`    | `solid-obj`       |
| `serde_yaml`    | `solid-gltf`      |
| …               | `solid-fbx`, `solid-usd`, … |

---

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         Application                         │
│  registry.load_file("hero.fbx") → Scene                     │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                    solid-rs  (this crate)                   │
│                                                             │
│  ┌──────────────────┐  ┌───────────────┐  ┌─────────────┐   │
│  │  Scene Graph     │  │    Traits     │  │  Registry   │   │
│  │  Scene           │  │  Loader       │  │             │   │
│  │  Node            │  │  Saver        │  │  register   │   │
│  │  Mesh            │  │  SceneVisitor │  │  load_file  │   │
│  │  Material        │  │  FormatInfo   │  │  save_file  │   │
│  │  Texture …       │  └───────────────┘  └─────────────┘   │
│  └──────────────────┘                                       │
└─────────────────────────────────────────────────────────────┘
         ▲                   ▲                    ▲
         │ uses              │ implements         │ implements
┌────────┴──────┐   ┌────────┴──────┐   ┌─────────┴──────┐
│   solid-obj   │   │  solid-gltf   │   │   solid-fbx    │
│  ObjLoader    │   │  GltfLoader   │   │  FbxLoader     │
│  ObjSaver     │   │  GltfSaver    │   │  FbxSaver      │
└───────────────┘   └───────────────┘   └────────────────┘
```

---

## Core Crate (`solid-rs`)

### Scene Data Model

All scene data is represented by plain Rust structs in the `solid_rs::scene`
module.  The top-level [`Scene`] owns flat `Vec`s of every object type; nodes
and primitives reference objects by index, not by pointer.

```
Scene
├── nodes:      Vec<Node>
├── meshes:     Vec<Mesh>
├── materials:  Vec<Material>
├── textures:   Vec<Texture>
├── images:     Vec<Image>
├── cameras:    Vec<Camera>
├── lights:     Vec<Light>
├── skins:      Vec<Skin>
└── animations: Vec<Animation>
```

### Traits

| Trait           | Implemented by | Purpose                        |
|:--------------- |:-------------- |:------------------------------ |
| `Loader`        | format crate   | `read → Scene`                 |
| `Saver`         | format crate   | `Scene → write`                |
| `SceneVisitor`  | format crate   | Walk a scene without cloning   |

### Registry

[`Registry`] is a runtime container for `dyn Loader` and `dyn Saver` objects.
Applications populate it at startup then call `load_file` / `save_file`.

### SceneBuilder

[`SceneBuilder`] is the ergonomic API that format-crate loaders use to
assemble a `Scene` incrementally during parsing.  It manages node-ID
allocation and keeps the invariant that all cross-references are valid.

### Extensions

[`Extensions`] is a typed property bag (keyed by `TypeId`) attached to every
scene object.  Format crates insert their own structs without any central
registration step — exactly like `serde`'s `#[serde(skip)]` and custom
deserialise logic.

---

## Format Crate Pattern

A minimal format crate:

```toml
# Cargo.toml
[dependencies]
solid-rs = "0.1"
```

```rust
// src/lib.rs
use solid_rs::prelude::*;
use solid_rs::traits::ReadSeek;

pub struct MyLoader;

static INFO: FormatInfo = FormatInfo {
    name: "My Format", id: "myfmt",
    extensions: &["myfmt"], mime_types: &[],
    can_load: true, can_save: false, spec_version: None,
};

impl Loader for MyLoader {
    fn load(&self, reader: &mut dyn ReadSeek, opts: &LoadOptions) -> Result<Scene> {
        let mut b = SceneBuilder::new();
        // … parse reader, call b.push_mesh() etc. …
        Ok(b.build())
    }
    fn format_info(&self) -> &FormatInfo { &INFO }
}
```

---

## Design Decisions

### Why index-based references instead of `Arc<T>`?

Index references keep `Scene` `Clone`-able, serialisable, and cache-friendly.
They also eliminate reference-count overhead during batch processing.

### Why a fixed data model rather than user-defined types (like serde)?

3-D formats share a well-defined set of concepts (meshes, materials, textures,
animation) that map to a common interchange model.  The `Extensions` bag
handles anything that falls outside the standard model.

### Why `ReadSeek` for loaders?

Many binary formats (FBX, GLB, USDZ) require random access to read headers
and index tables.  Requiring both `Read` and `Seek` makes this possible
without buffering the entire file in memory.

Because generic methods (`fn load<R: Read + Seek>`) make a trait non-dyn-compatible
— preventing `Arc<dyn Loader>` and dynamic dispatch in the registry —
SolidRS instead declares a `ReadSeek` supertrait:

```rust
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}
```

Loaders then accept `&mut dyn ReadSeek`, which is a single fat pointer and fully
object-safe.  Callers pass any `T: Read + Seek` (files, cursors, memory-mapped
regions) without wrapping.
