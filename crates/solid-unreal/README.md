# solid-unreal

Unreal Engine package (`.uasset` / `.umap`) loader for [solid-rs](https://crates.io/crates/solid-rs).

Converts UE4/UE5 packages into the Solid3D scene model for export to any supported format (FBX, glTF, OBJ, etc.).

## Capabilities

| Feature | Status | Notes |
|---------|--------|-------|
| Package structure (header, names, imports, exports) | ✅ | Via [`uasset`](https://github.com/Far-Beyond-Pulsar/UASSET) |
| BSP geometry (CubeBuilder) | ✅ | Inline vertex data from map brushes |
| **Cooked** `UStaticMesh` (`FStaticMeshRenderData`) | ✅ | Vertex positions, normals, tangents, UVs, colors |
| **Uncooked** `UStaticMesh` (editor meshes) | ❌ | No baked render data — skip or pre-cook |
| `UTexture2D` (mip data extraction) | ✅ | Bulk data + PlatformData parsing |
| `UMaterial` / `UMaterialInstanceConstant` (PBR mapping) | ✅ | Scalar, vector, texture parameter mapping |
| `UWorld` / `ULevel` actor hierarchy | ✅ | Actor transforms, components, scene graph |
| `FStaticMeshSection` (per-section materials) | ✅ | NumTriangles, first index, vertex ranges |
| Property tags (UE4.20+ GUID format) | ✅ | Version-aware via `uasset::Archive::file_version` |
| BSP geometry (Model/Polys) | ❌ | Not yet implemented |
| Skeletal meshes | ❌ | Not yet implemented |
| Landscapes | ❌ | Not yet implemented |

## How to maximise results

Solid3D's Unreal loader works best with **cooked** assets. Cooked assets have baked `FStaticMeshRenderData` containing the vertex buffers needed for mesh extraction. Uncooked (editor) assets store geometry through the Derived Data Cache and `MeshDescription` properties, which are not directly supported.

### For best results: Pre-cook your assets

In the Unreal Editor:

1. **Open your project** in the Unreal Editor
2. **Build the static meshes**: Select your meshes in the Content Browser → right-click → **Actions → Build**
3. **Cook the content**: File → Package → your target platform
4. **Use the cooked output**: Navigate to your project's `Saved/Cooked/[Platform]/[ProjectName]/Content/` directory
5. **Convert**: `unreal-to-fbx Saved/Cooked/.../MyLevel.umap output.fbx`

Alternatively, use the **Save Cooked Content** option in the editor's Project Settings to ensure all meshes are pre-baked.

### What happens with uncooked assets

| Scenario | Result |
|----------|--------|
| `.umap` with BSP brushes | ✅ Extracts inline brush geometry |
| `.umap` with uncooked `UStaticMesh` references | ⚠️ Mesh actors appear as empty nodes |
| Cooked `.uasset` (from shipped game) | ✅ Full mesh extraction |
| Uncooked `.uasset` (editor project) | ❌ No geometry — use pre-cooking above |

### Exporting a level with static meshes

For a map like the UE4 First Person example:

```
# Cooked map → full geometry extraction
cargo run -p unreal-to-fbx -- "path/to/cooked/Map.umap" "output.fbx"

# Uncooked → only BSP brushes
cargo run -p unreal-to-fbx -- "path/to/uncooked/Map.umap" "output.fbx"
```

## Quick start

```rust
use solid_rs::registry::Registry;
use solid_unreal::UnrealLoader;

let mut registry = Registry::new();
registry.register_loader(UnrealLoader);

let scene = registry.load_file("level.umap")?;
println!("Loaded {} meshes", scene.meshes.len());
```

## Architecture

The loader uses the [`uasset`](https://github.com/Far-Beyond-Pulsar/UASSET) crate for all package structure parsing (magic, versions, name table, import/export tables). Asset content decoding (properties, render data, textures) is handled by `solid-unreal`'s format-specific parsers, which use `uasset::Archive` for version-aware binary reading.

For cooked `UStaticMesh` assets, the render data parser follows the `FStaticMeshRenderData` layout as implemented by [CUE4Parse](https://github.com/FabianFG/CUE4Parse), including:
- `FStripDataFlags` for optional data sections
- Per-LOD sections with `NumTriangles` (×3 for index count)
- `FPositionVertexBuffer` (f32 or half-float vertices)
- `FStaticMeshVertexBuffer` (packed normals/tangents/UVs)
- `FColorVertexBuffer`
- `FRawStaticIndexBuffer` (16 or 32-bit indices)
