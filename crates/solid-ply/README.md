# solid-ply

Stanford PLY format loader and saver for the [SolidRS](https://github.com/Far-Beyond-Pulsar/solid-rs) ecosystem.

## Feature matrix

| Feature                    | Status |
|----------------------------|--------|
| ASCII load                 | ✅     |
| Binary little-endian load  | ✅     |
| Binary big-endian load     | ✅     |
| ASCII save                 | ✅     |
| Binary little-endian save  | ✅     |
| Point clouds               | ✅     |
| N-gon fan triangulation    | ✅     |
| Normals, UVs, vertex color | ✅     |

## Installation

```toml
[dependencies]
solid-ply = "0.1"
```

## Quick start

### Load a PLY file

```rust
use solid_ply::PlyLoader;
use solid_rs::prelude::*;
use std::fs::File;
use std::io::BufReader;

let file = File::open("mesh.ply")?;
let mut reader = BufReader::new(file);
let loader = PlyLoader;
let scene = loader.load(&mut reader, &LoadOptions::default())?;
println!("{} vertices", scene.meshes[0].vertices.len());
```

### Save ASCII PLY

```rust
use solid_ply::PlySaver;
use solid_rs::prelude::*;
use std::fs::File;
use std::io::BufWriter;

let file = File::create("out.ply")?;
let mut writer = BufWriter::new(file);
let saver = PlySaver;
saver.save(&scene, &mut writer, &SaveOptions::default())?;
```

### Save binary little-endian PLY

```rust
use solid_ply::PlySaver;
use solid_rs::prelude::*;
use std::fs::File;
use std::io::BufWriter;

let file = File::create("out_binary.ply")?;
let mut writer = BufWriter::new(file);
PlySaver::save_binary_le(&scene, &mut writer, &SaveOptions::default())?;
```

## PLY format reference

### Header layout

```
ply
format ascii 1.0            (or binary_little_endian / binary_big_endian)
comment ...
element vertex 1234
property float x
property float y
property float z
property float nx
property float ny
property float nz
property float s
property float t
property uchar red
property uchar green
property uchar blue
property uchar alpha
element face 456
property list uchar int vertex_indices
end_header
```

### Property type table

| PLY token           | Rust type |
|---------------------|-----------|
| `char` / `int8`     | `i8`      |
| `uchar` / `uint8`   | `u8`      |
| `short` / `int16`   | `i16`     |
| `ushort` / `uint16` | `u16`     |
| `int` / `int32`     | `i32`     |
| `uint` / `uint32`   | `u32`     |
| `float` / `float32` | `f32`     |
| `double` / `float64`| `f64`     |

## Vertex attribute mapping

| PLY property                        | `solid_rs::Vertex` field  |
|-------------------------------------|---------------------------|
| `x`, `y`, `z`                       | `position`                |
| `nx`, `ny`, `nz`                    | `normal`                  |
| `s` / `u` / `texture_u`            | `uvs[0].x`                |
| `t` / `v` / `texture_v`            | `uvs[0].y`                |
| `red`/`r`, `green`/`g`, `blue`/`b` | `colors[0].xyz`           |
| `alpha` / `a`                       | `colors[0].w`             |

Colors stored as `uchar` (0–255) are normalised to `[0, 1]` on load and
denormalised back to `[0, 255]` on save.

## Crate layout

```
crates/solid-ply/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs      — crate root, PLY_FORMAT static, public re-exports
    ├── header.rs   — header parser (PlyHeader, Element, Property, ScalarType)
    ├── loader.rs   — PlyLoader (ASCII + binary LE/BE)
    └── saver.rs    — PlySaver  (ASCII default, binary LE via save_binary_le)
```
