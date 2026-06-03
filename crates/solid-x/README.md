# solid-x

DirectX `.x` format support for `solid-rs`.

## Status

- Load: ASCII `.x` (`xof ....txt ....`)
- Save: ASCII `.x`
- Unsupported: binary/compressed `.x` variants (`bin`, `tzip`, `bzip`)

## Usage

```rust
use solid_rs::registry::Registry;
use solid_x::{XLoader, XSaver};

let mut reg = Registry::new();
reg.register_loader(XLoader);
reg.register_saver(XSaver);
```
