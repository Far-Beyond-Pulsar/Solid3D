# SolidRS

A **generic 3D model loading and saving library** for Rust — designed on the
same split as `serde` / `serde_json`:

| | Serialisation | 3D models |
|---|---|---|
| **Core** | `serde` | `solid-rs` |
| **Format** | `serde_json`, `serde_yaml` … | `solid-fbx`, `solid-obj` … |

`solid-rs` defines the scene IR, the `Loader`/`Saver` traits, and the format
`Registry`. Format crates implement those traits for a specific file format and
are pulled in à-la-carte.

---

## Crate Ecosystem

| Crate | Status | Description |
|---|---|---|
| [`solid-rs`](crates/solid-rs) | ✅ stable | Core scene types, traits, registry |
| [`solid-fbx`](crates/solid-fbx) | ✅ stable | Autodesk FBX binary + ASCII loader; ASCII 7.4 saver; cameras, lights, vertex colours, skinning, animation |
| [`solid-obj`](crates/solid-obj) | ✅ stable | Wavefront OBJ / MTL loader + saver; smoothing groups; PBR MTL extensions |
| [`solid-gltf`](crates/solid-gltf) | ✅ stable | glTF 2.0 JSON + GLB load + save; skinning; animation; KHR_lights_punctual |
| [`solid-stl`](crates/solid-stl) | ✅ stable | STL binary + ASCII load + save; smooth normals; VisCAM vertex colours |
| [`solid-ply`](crates/solid-ply) | ✅ stable | PLY ASCII + binary LE/BE load; ASCII + binary LE/BE save; double precision; point clouds; multi-UV; tangents |
| `solid-usd` | 🔜 planned | OpenUSD / USDA / USDC loader + saver |

---

## Quick Start

Add the core crate plus whichever format crates you need:

```toml
[dependencies]
solid-rs   = "0.1"
solid-fbx  = "0.1"   # Autodesk FBX
solid-obj  = "0.1"   # Wavefront OBJ
solid-gltf = "0.1"   # glTF 2.0 / GLB
solid-stl  = "0.1"   # Stereolithography STL
solid-ply  = "0.1"   # Stanford PLY
```

### Load a file

```rust
use solid_rs::prelude::*;
use solid_rs::registry::Registry;
use solid_fbx::FbxLoader;

fn main() -> solid_rs::Result<()> {
    let mut registry = Registry::new();
    registry.register_loader(FbxLoader);

    let scene = registry.load_file("model.fbx")?;
    println!("Loaded {} mesh(es), {} material(s)",
        scene.meshes.len(),
        scene.materials.len());
    Ok(())
}
```

### Save a file

```rust
use solid_rs::prelude::*;
use solid_rs::registry::Registry;
use solid_obj::{ObjLoader, ObjSaver};

fn main() -> solid_rs::Result<()> {
    let mut registry = Registry::new();
    registry.register_loader(ObjLoader);
    registry.register_saver(ObjSaver);

    let scene = registry.load_file("input.obj")?;
    registry.save_file(&scene, "output.obj")?;
    Ok(())
}
```

### Convert between formats

```rust
use solid_rs::prelude::*;
use solid_rs::registry::Registry;
use solid_fbx::FbxLoader;
use solid_obj::ObjSaver;

fn main() -> solid_rs::Result<()> {
    let mut registry = Registry::new();
    registry.register_loader(FbxLoader);
    registry.register_saver(ObjSaver);

    let opts = LoadOptions { triangulate: true, ..Default::default() };
    let scene = registry.load_file_with_options("scene.fbx", &opts)?;
    registry.save_file(&scene, "scene.obj")?;
    Ok(())
}
```

A ready-to-run conversion example lives in
[`examples/fbx-to-obj`](examples/fbx-to-obj/README.md):

```bash
cargo run -p fbx-to-obj -- input.fbx output.obj
```

---

## Architecture

```
┌────────────────────────────────────────────────────┐
│                     solid-rs                       │
│                                                    │
│  Scene ── Node ── Mesh ── Material ── Texture      │
│  Camera ── Light ── Animation ── Skin              │
│                                                    │
│  trait Loader   trait Saver   Registry             │
│  SceneBuilder   LoadOptions   SaveOptions          │
└──────────────────────┬─────────────────────────────┘
                       │  implements
       ┌───────────────┼───────────────────┐
       ▼               ▼                   ▼
   solid-fbx       solid-obj          solid-gltf
   solid-stl       solid-ply          solid-usd …
```

### Scene IR

The intermediate representation is a **flat, index-based scene graph** —
similar in spirit to glTF's document model:

```
Scene
 ├── nodes:     Vec<Node>          (parent refs by NodeId index)
 ├── meshes:    Vec<Mesh>          (Mesh.primitives: Vec<Primitive>)
 ├── materials: Vec<Material>      (PBR metallic-roughness)
 ├── textures:  Vec<Texture>       (URI or embedded bytes)
 ├── images:    Vec<Image>
 ├── cameras:   Vec<Camera>
 ├── lights:    Vec<Light>
 ├── animations:Vec<Animation>
 └── skins:     Vec<Skin>
```

Every cross-reference is an integer index, keeping `Scene` fully `Clone`-able
without `Arc` or reference cycles.

### Traits

```rust
// dyn-compatible — usable as Arc<dyn Loader>
pub trait Loader: Send + Sync {
    fn format_info(&self) -> &'static FormatInfo;
    fn load(&self, reader: &mut dyn ReadSeek, options: &LoadOptions)
        -> Result<Scene>;
}

pub trait Saver: Send + Sync {
    fn format_info(&self) -> &'static FormatInfo;
    fn save(&self, scene: &Scene, writer: &mut dyn Write, options: &SaveOptions)
        -> Result<()>;
}
```

Both traits are **dyn-compatible** — format drivers can be stored in the
registry as boxed trait objects and dispatched at runtime by file extension or
MIME type.

---

## Building

```bash
# Build everything
cargo build --workspace

# Run the full test suite (~350 tests)
cargo test --workspace

# Run the FBX → OBJ converter example
cargo run -p fbx-to-obj -- input.fbx output.obj
```

### Format support matrix

| Format | Load | Save | Notes |
|---|---|---|---|
| FBX (binary) | ✅ | — | 6.1–7.4, 32 + 64-bit offsets |
| FBX (ASCII) | ✅ | ✅ | 7.4; cameras, lights, vertex colours, skinning, animation |
| OBJ / MTL | ✅ | ✅ | N-gon fan triangulation; smoothing groups; PBR MTL |
| glTF 2.0 JSON | ✅ | ✅ | Skinning, animation, KHR_lights_punctual, sparse accessors |
| GLB | ✅ | ✅ | Binary glTF; `GltfSaver::save_glb()` |
| STL binary | ✅ | ✅ | Vertex dedup; smooth normals; VisCAM vertex colours |
| STL ASCII | ✅ | ✅ | `StlSaver::save_ascii()` |
| PLY ASCII | ✅ | ✅ | N-gon fan triangulation; point clouds; multi-UV; tangents |
| PLY binary LE | ✅ | ✅ | `PlySaver::save_binary_le()` / `save_with_precision()` |
| PLY binary BE | ✅ | ✅ | `PlySaver::save_binary_be()` |

---

## Format Feature Details

Legend: ✅ supported · ⚠️ partial · ❌ not supported · — not applicable to this format

---

### FBX — Autodesk Filmbox ([`solid-fbx`](crates/solid-fbx))

Extensions: `.fbx` · MIME: `model/fbx`

| Feature | Load | Save | Notes |
|---|---|---|---|
| **Encoding** | | | |
| Binary FBX | ✅ | — | v6.1–v7.7; auto-detects 32-bit (< 7500) vs 64-bit offsets |
| ASCII FBX | ✅ | ✅ | v7.4 format |
| **Geometry** | | | |
| Positions | ✅ | ✅ | |
| Normals (`ByPolygonVertex` / `ByVertex`) | ✅ | ✅ | |
| UV coordinates (channel 0) | ✅ | ✅ | V-axis flipped on load/save |
| Vertex colours (`LayerElementColor`) | ✅ | ✅ | Direct + IndexToDirect |
| Tangents | ❌ | ❌ | |
| N-gon triangulation (`PolygonVertexIndex`) | ✅ | ✅ | Fan method |
| Per-polygon material (`LayerElementMaterial`) | ✅ | ✅ | `AllSame` + `ByPolygon` |
| **Scene graph** | | | |
| Node hierarchy (parent / child) | ✅ | ✅ | Topological sort handles arbitrary depth |
| Local TRS transforms | ✅ | ✅ | Euler → Quat on load; Quat → Euler XYZ on save |
| **Materials** | | | |
| Diffuse colour | ✅ | ✅ | |
| Emissive colour + factor | ✅ | ✅ | |
| Roughness (from `Shininess`) | ✅ | ✅ | `sqrt(2/(Ns+2))` conversion |
| Metallic (from `ReflectionFactor`) | ✅ | ✅ | |
| Alpha / opacity | ✅ | ✅ | `TransparencyFactor` / `Opacity` |
| **Textures** | | | |
| Diffuse texture | ✅ | ✅ | Filename / URI |
| Normal map | ✅ | ✅ | |
| Emissive / roughness textures | ❌ | ❌ | |
| **Lights** | | | |
| Point light | ✅ | ✅ | Colour, intensity, range |
| Directional light | ✅ | ✅ | |
| Spot light | ✅ | ✅ | Inner + outer cone angle |
| Area light | ❌ | ❌ | |
| **Cameras** | | | |
| Perspective camera | ✅ | ✅ | FOV, near/far planes |
| Orthographic camera | ❌ | ❌ | |
| **Skinning** | | | |
| Vertex weights (up to 4 influences) | ✅ | ✅ | Top-4 normalised |
| Inverse bind-pose matrices | ✅ | ✅ | From `TransformLink` |
| **Animation** | | | |
| Translation / rotation / scale keyframes | ✅ | ✅ | Linear interpolation |
| Euler rotation → quaternion conversion | ✅ | ✅ | XYZ order |
| Multi-track animation stacks | ✅ | ✅ | One `Animation` per `AnimationStack` |
| Morph target weights | ❌ | ❌ | |

---

### OBJ — Wavefront ([`solid-obj`](crates/solid-obj))

Extensions: `.obj`, `.mtl` · MIME: `model/obj`

| Feature | Load | Save | Notes |
|---|---|---|---|
| **Geometry** | | | |
| Positions (`v`) | ✅ | ✅ | |
| Normals (`vn`) | ✅ | ✅ | |
| UV coordinates (`vt`) | ✅ | ✅ | |
| Triangles (`f 1 2 3`) | ✅ | ✅ | |
| Quads & N-gons | ✅ | — | Fan-triangulated on load |
| Negative (relative) indices | ✅ | — | |
| **Groups** | | | |
| Object groups (`o`) | ✅ | ✅ | One mesh per object |
| Named groups (`g`) | ✅ | ✅ | One primitive per group |
| Smoothing groups (`s`) | ⚠️ | — | Parsed but not used |
| **Materials (MTL)** | | | |
| External `.mtl` file | ✅ | ✅ | Resolved from `LoadOptions::base_dir` |
| Embedded MTL block | — | ✅ | Written inline in `.obj` |
| Diffuse colour (`Kd`) | ✅ | ✅ | |
| Specular colour (`Ks`) | ✅ | ✅ | → `metallic_factor` |
| Emissive colour (`Ke`) | ✅ | ✅ | |
| Shininess (`Ns`) | ✅ | ✅ | → `roughness_factor` |
| Opacity (`d` / `Tr`) | ✅ | ✅ | |
| Diffuse texture (`map_Kd`) | ✅ | ✅ | |
| Normal map (`map_Bump` / `bump`) | ✅ | ✅ | |
| PBR extensions (`map_Pr`, `map_Pm`) | ⚠️ | — | Parsed, not mapped |
| **Scene graph** | | | |
| Node hierarchy | — | — | OBJ has no hierarchy |
| Transforms | — | — | |
| Cameras / lights / skinning / animation | — | — | |

---

### glTF 2.0 — Khronos ([`solid-gltf`](crates/solid-gltf))

Extensions: `.gltf`, `.glb` · MIME: `model/gltf+json`, `model/gltf-binary`

| Feature | Load | Save | Notes |
|---|---|---|---|
| **Encoding** | | | |
| JSON (`.gltf`) | ✅ | ✅ | |
| Binary GLB | ✅ | ✅ | `GltfSaver::save_glb()` |
| External buffer URIs | ✅ | — | Resolved from `base_dir` |
| Base64 data URIs | ✅ | ✅ | Embedded in JSON |
| **Geometry** | | | |
| Positions (`POSITION`) | ✅ | ✅ | |
| Normals (`NORMAL`) | ✅ | ✅ | |
| Tangents (`TANGENT`) | ✅ | ✅ | |
| UV channels (`TEXCOORD_0`–`7`) | ✅ | ✅ | Up to 8 channels |
| Vertex colours (`COLOR_0`) | ✅ | ✅ | |
| Accessor types: FLOAT / U8 / U16 / U32 | ✅ | ✅ | Normalised reads |
| Sparse accessors | ❌ | — | |
| **Scene graph** | | | |
| Node hierarchy | ✅ | ✅ | |
| TRS transforms | ✅ | ✅ | |
| Matrix transforms | ✅ | — | Decomposed on load |
| **Materials (PBR metallic-roughness)** | | | |
| Base colour factor + texture | ✅ | ✅ | |
| Metallic / roughness factor + texture | ✅ | ✅ | |
| Normal map (+ scale) | ✅ | ✅ | |
| Occlusion map (+ strength) | ✅ | ✅ | |
| Emissive factor + texture | ✅ | ✅ | |
| Alpha modes (OPAQUE / MASK / BLEND) | ✅ | ✅ | |
| Double-sided flag | ✅ | ✅ | |
| **Cameras** | | | |
| Perspective | ✅ | ✅ | |
| Orthographic | ✅ | ✅ | |
| **Skinning** | | | |
| Joints + weights (`JOINTS_0`, `WEIGHTS_0`) | ✅ | — | Loaded into `SkinWeights` |
| Inverse bind matrices | ✅ | — | |
| **Animation** | | | |
| Translation / rotation / scale samplers | ✅ | — | |
| LINEAR / STEP / CUBICSPLINE | ✅ | — | |
| Morph target weights | ❌ | — | |
| **Lighting** | | | |
| Cameras attached to nodes | ✅ | ✅ | |
| `KHR_lights_punctual` extension | ❌ | ❌ | |

---

### STL — Stereolithography ([`solid-stl`](crates/solid-stl))

Extensions: `.stl` · MIME: `model/stl`, `application/sla`

| Feature | Load | Save | Notes |
|---|---|---|---|
| **Encoding** | | | |
| Binary STL | ✅ | ✅ | Default save format |
| ASCII STL | ✅ | ✅ | `StlSaver::save_ascii()` |
| Auto-detect binary vs ASCII | ✅ | — | Triangle-count checksum method |
| **Geometry** | | | |
| Positions | ✅ | ✅ | |
| Face normals | ✅ | ✅ | Stored per-triangle; recomputed on save |
| Vertex deduplication | ✅ | — | `HashMap<[u32;3], u32>` bit-cast dedup |
| Vertex normals | ⚠️ | — | Face normal assigned to all 3 vertices |
| UV / vertex colours / tangents | — | — | Not supported by format |
| **Scene graph** | | | |
| Scene name (from `solid <name>`) | ✅ | ✅ | ASCII only |
| Node hierarchy / transforms | — | — | Not supported by format |
| Materials / textures | — | — | Not supported by format |
| Cameras / lights / skinning / animation | — | — | Not supported by format |

---

### PLY — Stanford Polygon ([`solid-ply`](crates/solid-ply))

Extensions: `.ply` · MIME: `model/ply`

| Feature | Load | Save | Notes |
|---|---|---|---|
| **Encoding** | | | |
| ASCII PLY | ✅ | ✅ | |
| Binary little-endian | ✅ | ✅ | `PlySaver::save_binary_le()` |
| Binary big-endian | ✅ | — | Read-only |
| **Property types** | | | |
| `char` / `uchar` (int8 / uint8) | ✅ | ✅ | |
| `short` / `ushort` (int16 / uint16) | ✅ | — | |
| `int` / `uint` (int32 / uint32) | ✅ | ✅ | |
| `float` (float32) | ✅ | ✅ | |
| `double` (float64) | ✅ | — | |
| List properties | ✅ | ✅ | `property list uchar uint vertex_indices` |
| **Geometry** | | | |
| Positions (`x`, `y`, `z`) | ✅ | ✅ | |
| Normals (`nx`, `ny`, `nz`) | ✅ | ✅ | Written only if present |
| UV coordinates (`s`/`u`, `t`/`v`, `texture_u/v`) | ✅ | ✅ | |
| Vertex colours (`red`, `green`, `blue`, `alpha`) | ✅ | ✅ | `uchar` 0–255 ↔ float 0–1 |
| Triangles | ✅ | ✅ | |
| N-gon fan triangulation | ✅ | — | |
| Point clouds (no faces) | ✅ | — | Loaded as mesh with no primitives |
| **Scene graph** | | | |
| Node hierarchy / transforms | — | — | Not supported by format |
| Materials / textures | — | — | Not supported by format |
| Cameras / lights / skinning / animation | — | — | Not supported by format |

---

## Implementing a Format Crate

See the step-by-step guides in [`docs/`](docs/):

| Document | Topic |
|---|---|
| [`getting-started.md`](docs/getting-started.md) | Workspace setup, first load |
| [`implementing-a-loader.md`](docs/implementing-a-loader.md) | Writing a `Loader` |
| [`implementing-a-saver.md`](docs/implementing-a-saver.md) | Writing a `Saver` |
| [`traits-reference.md`](docs/traits-reference.md) | Full trait API reference |
| [`scene-graph.md`](docs/scene-graph.md) | Scene IR deep-dive |
| [`geometry.md`](docs/geometry.md) | Vertex, Primitive, Mesh |
| [`format-registry.md`](docs/format-registry.md) | Registry & dispatch |
| [`error-handling.md`](docs/error-handling.md) | `SolidError` & `Result` |
| [`architecture.md`](docs/architecture.md) | Design decisions |

Minimal loader skeleton:

```rust
use solid_rs::prelude::*;
use solid_rs::traits::{Loader, ReadSeek};

pub static MY_FORMAT: FormatInfo = FormatInfo {
    name:         "My Format",
    id:           "my-format",
    extensions:   &["mfmt"],
    mime_types:   &["model/x-my-format"],
    can_load:     true,
    can_save:     false,
    spec_version: None,
};

pub struct MyLoader;

impl Loader for MyLoader {
    fn format_info(&self) -> &'static FormatInfo { &MY_FORMAT }

    fn load(&self, reader: &mut dyn ReadSeek, _opts: &LoadOptions)
        -> Result<Scene>
    {
        let mut builder = solid_rs::builder::SceneBuilder::new();
        // … parse reader, call builder methods …
        Ok(builder.build())
    }
}
```

---

## License

MIT — see [`LICENSE`](LICENSE).
