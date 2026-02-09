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
use tealeaf::{ToTeaLeaf, ToTeaLeafExt};

#[derive(ToTeaLeaf)]
struct Config {
    name: String,
    port: u16,
    debug: bool,
}

let config = Config { name: "app".into(), port: 8080, debug: true };
let doc = config.to_tealeaf_doc("config");
let text = doc.to_tl_with_schemas();
```

### `FromTeaLeaf`

Reconstructs a Rust struct from a TeaLeaf `Value`:

```rust
use tealeaf::{FromTeaLeaf, Value};

#[derive(FromTeaLeaf)]
struct Config {
    name: String,
    port: u16,
    debug: bool,
}

let value = /* parsed TeaLeaf value */;
let config = Config::from_tealeaf_value(&value)?;
```

### Field Attributes

| Attribute | Description |
|-----------|-------------|
| `#[tealeaf(rename = "name")]` | Use a different field name in TeaLeaf output |
| `#[tealeaf(skip)]` | Skip this field during conversion |
| `#[tealeaf(optional)]` | Mark field as nullable in schema |
| `#[tealeaf(type = "timestamp")]` | Override the TeaLeaf type in schema |
| `#[tealeaf(flatten)]` | Flatten nested struct fields into the parent |
| `#[tealeaf(default)]` | Use `Default::default()` when deserializing a missing field |
| `#[tealeaf(default = "expr")]` | Use a custom default expression for missing fields |

### Container Attributes

| Attribute | Description |
|-----------|-------------|
| `#[tealeaf(rename = "name")]` | Override the schema name |
| `#[tealeaf(root_array)]` | Mark as a root-level array |
| `#[tealeaf(key = "name")]` | Set the data key when serializing to a document |

## License

MIT

## Links

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [Derive Macros Guide](https://krishjag.github.io/tealeaf/rust/derive-macros.html)
