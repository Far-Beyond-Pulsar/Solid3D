# solid-stl

STL (Stereolithography) binary and ASCII loader and saver for the [SolidRS](https://github.com/Far-Beyond-Pulsar/solid-rs) ecosystem.

## Feature Matrix

| Feature                  | Supported |
|--------------------------|-----------|
| Binary load              | ✅        |
| ASCII load               | ✅        |
| Binary save              | ✅        |
| ASCII save               | ✅        |
| Vertex dedup             | ✅        |
| Smooth vertex normals    | ✅        |
| Vertex colors (VisCAM)   | ✅        |
| Multiple meshes (binary) | ✅        |
| Multiple meshes (ASCII)  | ✅        |

## Installation

```toml
[dependencies]
solid-stl = "0.1"
```

## Quick Start

### Load an STL file

```rust
use solid_stl::StlLoader;
use solid_rs::prelude::*;
use std::fs::File;
use std::io::BufReader;

let mut reader = BufReader::new(File::open("model.stl")?);
let options = LoadOptions::default();
let scene = StlLoader.load(&mut reader, &options)?;
println!("Loaded {} meshes", scene.meshes.len());
```

### Save binary STL (default)

```rust
use solid_stl::StlSaver;
use solid_rs::prelude::*;
use std::fs::File;
use std::io::BufWriter;

let mut writer = BufWriter::new(File::create("out.stl")?);
let options = SaveOptions::default();
StlSaver.save(&scene, &mut writer, &options)?;
```

### Save ASCII STL

```rust
use solid_stl::StlSaver;
use solid_rs::prelude::*;
use std::fs::File;
use std::io::BufWriter;

let mut writer = BufWriter::new(File::create("out_ascii.stl")?);
let options = SaveOptions::default();
StlSaver.save_ascii(&scene, &mut writer, &options)?;
```

## STL Format Notes

### Binary layout

| Offset | Size     | Description                        |
|--------|----------|------------------------------------|
| 0      | 80 bytes | Header (ignored on load)           |
| 80     | 4 bytes  | Triangle count (`u32` LE)          |
| 84+    | 50 bytes | Per triangle (repeated):           |
|        | 12 bytes | &nbsp;&nbsp;Normal `f32[3]`        |
|        | 12 bytes | &nbsp;&nbsp;Vertex 0 `f32[3]`      |
|        | 12 bytes | &nbsp;&nbsp;Vertex 1 `f32[3]`      |
|        | 12 bytes | &nbsp;&nbsp;Vertex 2 `f32[3]`      |
|        | 2 bytes  | &nbsp;&nbsp;Attribute count (`u16`)|

Binary detection: `80 + 4 + count × 50 == file_length`

### ASCII syntax

```
solid [name]
  facet normal nx ny nz
    outer loop
      vertex x y z
      vertex x y z
      vertex x y z
    endloop
  endfacet
  ...
endsolid [name]
```

### Vertex deduplication

STL has no shared vertices — every triangle carries 3 independent positions. On load, `solid-stl` deduplicates vertices using a `HashMap<[u32; 3], u32>` (f32 bits as key), typically reducing vertex count ~6× for closed meshes.

## Crate Layout

| File           | Contents                               |
|----------------|----------------------------------------|
| `src/lib.rs`   | Public re-exports, `STL_FORMAT` static |
| `src/parser.rs`| Binary + ASCII parse, `detect_binary`  |
| `src/loader.rs`| `StlLoader` implementing `Loader`      |
| `src/saver.rs` | `StlSaver` implementing `Saver`        |
