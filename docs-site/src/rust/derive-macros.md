# Derive Macros

The `tealeaf-derive` crate provides two proc-macros for automatic Rust struct ↔ TeaLeaf conversion.

## Setup

Enable the `derive` feature:

```toml
[dependencies]
tealeaf-core = { version = "2.0.0-beta.10", features = ["derive"] }
```

## ToTeaLeaf

Converts a Rust struct or enum to a TeaLeaf `Value`:

```rust
use tealeaf::{ToTeaLeaf, ToTeaLeafExt};

#[derive(ToTeaLeaf)]
struct Config {
    host: String,
    port: i32,
    debug: bool,
}

let config = Config { host: "localhost".into(), port: 8080, debug: true };

// Serialize to TeaLeaf text
let text = config.to_tl_string("config");
// @struct config (host: string, port: int, debug: bool)
// config: (localhost, 8080, true)

// Compile directly to binary
config.to_tlbx("config", "config.tlbx", true)?;

// Convert to JSON
let json = config.to_tealeaf_json("config")?;

// Get as Value
let value = config.to_tealeaf_value();

// Get schemas
let schemas = Config::collect_schemas();
```

## FromTeaLeaf

Deserializes a TeaLeaf `Value` back to a Rust struct:

```rust
use tealeaf::{Reader, FromTeaLeaf};

#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Config {
    host: String,
    port: i32,
    debug: bool,
}

let reader = Reader::open("config.tlbx")?;
let value = reader.get("config")?;
let config = Config::from_tealeaf_value(&value)?;
```

## Struct Example

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct User {
    id: i64,
    name: String,
    #[tealeaf(optional)]
    email: Option<String>,
    active: bool,
    #[tealeaf(rename = "join_date", type = "timestamp")]
    joined: i64,
}
```

This generates:
- Schema: `@struct user (id: int64, name: string, email: string?, active: bool, join_date: timestamp)`
- `ToTeaLeaf`: serializes to a positional tuple matching the schema
- `FromTeaLeaf`: deserializes from an object or struct-array row

## Enum Example

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Point,
}

let shapes = vec![
    Shape::Circle { radius: 5.0 },
    Shape::Rectangle { width: 10.0, height: 20.0 },
    Shape::Point,
];
```

Enum variants are serialized as tagged values:
```tl
shapes: [:circle {radius: 5.0}, :rectangle {width: 10.0, height: 20.0}, :point ~]
```

## Nested Structs

Structs can reference other `ToTeaLeaf`/`FromTeaLeaf` types:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Address {
    street: String,
    city: String,
    zip: String,
}

#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Person {
    name: String,
    home: Address,
    #[tealeaf(optional)]
    work: Option<Address>,
}
```

The `collect_schemas()` method automatically collects schemas from nested types.

## Collections

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Team {
    name: String,
    members: Vec<String>,      // []string
    scores: Vec<i32>,          // []int
    leads: Vec<Person>,        // []person (nested struct array)
}
```

## Supported Types

| Rust Type | TeaLeaf Type |
|-----------|-------------|
| `bool` | `bool` |
| `i8`, `i16`, `i32` | `int8`, `int16`, `int` |
| `i64` | `int64` |
| `u8`, `u16`, `u32` | `uint8`, `uint16`, `uint` |
| `u64` | `uint64` |
| `f32` | `float32` |
| `f64` | `float` |
| `String`, `&str` | `string` |
| `Vec<u8>` | `bytes` |
| `Vec<T>` | `[]T` |
| `Option<T>` | `T?` (nullable) |
| `IndexMap<String, T>` | object (order-preserving) |
| `HashMap<String, T>` | object |
| Custom struct (with derive) | named struct reference |

## See Also

- [Attributes Reference](./attributes.md) -- all `#[tealeaf(...)]` attributes
- [Builder API](./builder.md) -- manual document construction
