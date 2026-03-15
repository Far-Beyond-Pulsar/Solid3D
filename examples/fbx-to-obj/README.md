# fbx-to-obj

A command-line tool that converts an FBX file to a Wavefront OBJ file using the
SolidRS ecosystem (`solid-fbx` loader → `solid-rs` scene IR → `solid-obj` saver).

---

## Usage

```
fbx-to-obj [INPUT.fbx] [OUTPUT.obj]
```

| Argument | Default | Description |
|---|---|---|
| `INPUT.fbx` | `test.fbx` | Path to the source FBX file (binary or ASCII) |
| `OUTPUT.obj` | Same path with `.obj` extension | Destination OBJ file |

### Examples

```bash
# Convert the bundled test model (from the workspace root)
cargo run -p fbx-to-obj

# Convert a specific file, output next to it
cargo run -p fbx-to-obj -- my_model.fbx

# Explicit input and output paths
cargo run -p fbx-to-obj -- path/to/scene.fbx path/to/out.obj

# Release build for large files
cargo run --release -p fbx-to-obj -- huge_scene.fbx out.obj
```

---

## What it does

1. **Builds a `Registry`** and registers all four drivers:
   `FbxLoader`, `ObjLoader`, `FbxSaver`, `ObjSaver`.
2. **Loads** the FBX using `Registry::load_file_with_options`.
   - The loader auto-detects binary vs ASCII FBX from the magic bytes.
   - `triangulate: true` is passed so any N-gons are split before saving.
3. **Prints a scene summary** — node/mesh/material/texture counts plus per-mesh
   vertex and index counts.
4. **Saves** the scene to OBJ using `Registry::save_file`.
   - The saver writes an embedded MTL block inside the `.obj` file.

---

## Output format

The `.obj` file uses:
- A single MTL library block (embedded, not a separate `.mtl` file)
- One `o` object per scene mesh
- One `usemtl` / `g` group per `Primitive` on each mesh
- Per-vertex interleaved positions, normals, and UVs (channel 0)
- All faces triangulated

---

## Supported FBX features

| Feature | Status |
|---|---|
| Binary FBX (≥ 6.1, 32-bit and 64-bit offsets) | ✅ |
| ASCII FBX 7.4 | ✅ |
| Meshes with positions, normals, UVs | ✅ |
| Diffuse / emissive / roughness / metallic materials | ✅ |
| N-gon fan triangulation | ✅ |
| Scene hierarchy (parent → child nodes) | ✅ |
| Cameras, lights, skinning, animation | ⚠️ parsed but not exported to OBJ |

---

## Dependencies

```toml
solid-rs  = { path = "../../crates/solid-rs" }
solid-fbx = { path = "../../crates/solid-fbx" }
solid-obj = { path = "../../crates/solid-obj" }
```

Both format crates are thin wrappers around the `solid-rs` trait abstractions —
see their individual READMEs for deeper detail on the file-format internals.
