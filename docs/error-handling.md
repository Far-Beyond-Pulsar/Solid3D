# Error Handling

SolidRS uses a single unified error type — [`SolidError`] — for all loader
and saver operations.  This keeps error propagation simple for both format
crate authors and application code.

---

## `SolidError` Variants

| Variant              | When to use                                                          |
|:-------------------- |:-------------------------------------------------------------------- |
| `Io(io::Error)`      | Any underlying I/O failure                                           |
| `Parse(String)`      | Syntactically invalid data                                           |
| `UnsupportedFeature` | The format is recognised but a feature is unimplemented              |
| `UnsupportedFormat`  | No loader / saver registered for a given extension or format ID      |
| `InvalidReference`   | An index (node, mesh, material…) is out of range                     |
| `InvalidScene`       | Semantic error in the scene (e.g. cyclic hierarchy)                  |
| `Format { … }`       | Format-specific error from an extension crate                        |
| `Other(String)`      | Catch-all for anything else                                          |

---

## Constructor Helpers

```rust
use solid_rs::SolidError;

// Parse error with a message
let e = SolidError::parse("unexpected EOF at offset 42");

// Unsupported feature
let e = SolidError::unsupported("compressed textures");

// Format-specific error (preferred in extension crates)
let e = SolidError::format("fbx", "malformed node record header");

// Invalid cross-reference
let e = SolidError::invalid_ref("material index 9 out of bounds (only 3 materials)");
```

---

## The `Result<T>` Alias

```rust
pub type Result<T, E = SolidError> = std::result::Result<T, E>;
```

Import it from the prelude:

```rust
use solid_rs::prelude::*;  // includes Result and SolidError
```

---

## Propagating Errors in Format Crates

Use `?` to propagate `std::io::Error` directly — it converts automatically:

```rust
impl Loader for MyLoader {
    fn load(&self, reader: &mut dyn ReadSeek, _opts: &LoadOptions) -> Result<Scene> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;  // io::Error → SolidError::Io

        if &buf != b"MYFM" {
            return Err(SolidError::parse("invalid magic bytes"));
        }
        // …
        Ok(scene)
    }
}
```

Map third-party errors with `.map_err`:

```rust
let value = some_library::parse(data)
    .map_err(|e| SolidError::format("myformat", e.to_string()))?;
```

---

## Handling Errors in Application Code

```rust
use solid_rs::{SolidError, prelude::*};

match registry.load_file("model.xyz") {
    Ok(scene) => println!("Loaded {} meshes", scene.meshes.len()),

    Err(SolidError::UnsupportedFormat(msg)) =>
        eprintln!("No loader registered: {msg}"),

    Err(SolidError::Io(e)) =>
        eprintln!("I/O error: {e}"),

    Err(SolidError::Parse(msg)) =>
        eprintln!("Parse error: {msg}"),

    Err(e) =>
        eprintln!("Error: {e}"),
}
```

---

## Feature Compatibility Errors

If your loader encounters a feature it does not support (e.g. Draco mesh
compression, proprietary extensions), return `UnsupportedFeature` rather
than silently skipping it — this makes debugging much easier for users:

```rust
if flags & COMPRESSED_FLAG != 0 {
    return Err(SolidError::unsupported(
        "Draco mesh compression — reopen with a loader that supports it"
    ));
}
```
