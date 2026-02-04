# tealeaf-derive

Derive macros for converting Rust structs and enums to and from the TeaLeaf data format.

## Usage

This crate is typically used through the `derive` feature of `tealeaf-core`:

```toml
[dependencies]
tealeaf-core = { version = "2.0.0-beta.2", features = ["derive"] }
```

## Derive Macros

### `ToTeaLeaf`

Converts a Rust struct or enum into a TeaLeaf `Value` with automatic schema generation:

```rust
use tealeaf_core::{ToTeaLeaf, ToTeaLeafExt};

#[derive(ToTeaLeaf)]
struct Config {
    name: String,
    port: u16,
    debug: bool,
}

let config = Config { name: "app".into(), port: 8080, debug: true };
let (value, schemas) = config.to_tealeaf_with_schemas()?;
```

### `FromTeaLeaf`

Reconstructs a Rust struct from a TeaLeaf `Value`:

```rust
use tealeaf_core::{FromTeaLeaf, Value};

#[derive(FromTeaLeaf)]
struct Config {
    name: String,
    port: u16,
    debug: bool,
}

let value = /* parsed TeaLeaf value */;
let config = Config::from_tealeaf(&value)?;
```

### Field Attributes

| Attribute | Description |
|-----------|-------------|
| `#[tealeaf(rename = "name")]` | Use a different field name in TeaLeaf output |
| `#[tealeaf(skip)]` | Skip this field during conversion |

## License

MIT

## Links

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [Derive Macros Guide](https://krishjag.github.io/tealeaf/rust/derive-macros.html)
