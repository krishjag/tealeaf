# Rust Guide: Overview

TeaLeaf is written in Rust. The `tealeaf-core` crate provides the full API for parsing, compiling, reading, and converting TeaLeaf documents.

## Crates

| Crate | Description |
|-------|-------------|
| `tealeaf-core` | Core library: parser, compiler, reader, CLI, JSON conversion |
| `tealeaf-derive` | Proc-macro crate: `#[derive(ToTeaLeaf, FromTeaLeaf)]` |
| `tealeaf-ffi` | C-compatible FFI layer for language bindings |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tealeaf-core = { version = "2.0.0-beta.8", features = ["derive"] }
```

The `derive` feature pulls in `tealeaf-derive` for proc-macro support.

## Core Types

### `TeaLeaf`

The main document type:

```rust
use tealeaf::TeaLeaf;

// Parse from text
let doc = TeaLeaf::parse("name: Alice\nage: 30")?;

// Load from file
let doc = TeaLeaf::load("data.tl")?;

// Load from JSON
let doc = TeaLeaf::from_json(json_str)?;

// With schema inference
let doc = TeaLeaf::from_json_with_schemas(json_str)?;
```

### `Value`

The value enum representing all TeaLeaf types:

```rust
use tealeaf::Value;

pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(ObjectMap<String, Value>),  // IndexMap alias, preserves insertion order
    Map(Vec<(Value, Value)>),
    Ref(String),
    Tagged(String, Box<Value>),
    Timestamp(i64, i16),  // (unix_millis, tz_offset_minutes)
    JsonNumber(String),   // arbitrary-precision number (raw JSON decimal string)
}
```

### `Schema` and `Field`

Schema definitions:

```rust
use tealeaf::{Schema, Field, FieldType};

let schema = Schema {
    name: "user".to_string(),
    fields: vec![
        Field { name: "id".into(), field_type: FieldType { base: "int".into(), nullable: false, is_array: false } },
        Field { name: "name".into(), field_type: FieldType { base: "string".into(), nullable: false, is_array: false } },
        Field { name: "email".into(), field_type: FieldType { base: "string".into(), nullable: true, is_array: false } },
    ],
};
```

## Accessing Data

```rust
let doc = TeaLeaf::load("data.tl")?;

// Get a value by key
if let Some(Value::String(name)) = doc.get("name") {
    println!("Name: {}", name);
}

// Get a schema
if let Some(schema) = doc.schema("user") {
    for field in &schema.fields {
        println!("  {}: {}", field.name, field.field_type.base);
    }
}
```

## Output Operations

```rust
let doc = TeaLeaf::load("data.tl")?;

// Compile to binary
doc.compile("data.tlbx", true)?;  // true = enable compression

// Convert to JSON
let json = doc.to_json()?;         // pretty-printed
let json = doc.to_json_compact()?;  // minified

// Convert to TeaLeaf text (with schemas)
let text = doc.to_tl_with_schemas();

// Compact text (removes insignificant whitespace, ideal for LLM input)
let compact = doc.to_tl_with_schemas_compact();
```

## Conversion Traits

Two traits enable Rust struct ↔ TeaLeaf conversion:

```rust
pub trait ToTeaLeaf {
    fn to_tealeaf_value(&self) -> Value;
    fn collect_schemas() -> IndexMap<String, Schema>;
    fn tealeaf_field_type() -> FieldType;
}

pub trait FromTeaLeaf: Sized {
    fn from_tealeaf_value(value: &Value) -> Result<Self, ConvertError>;
}
```

These are typically derived via `#[derive(ToTeaLeaf, FromTeaLeaf)]` -- see [Derive Macros](./derive-macros.md).

## Extension Trait

`ToTeaLeafExt` provides convenience methods for any `ToTeaLeaf` implementor:

```rust
pub trait ToTeaLeafExt: ToTeaLeaf {
    fn to_tealeaf_doc(&self, key: &str) -> TeaLeaf;
    fn to_tl_string(&self, key: &str) -> String;
    fn to_tlbx(&self, key: &str, path: &str, compress: bool) -> Result<()>;
    fn to_tealeaf_json(&self, key: &str) -> Result<String>;
}
```

Example:

```rust
let user = User { id: 1, name: "Alice".into(), active: true };

// One-liner serialization
let text = user.to_tl_string("user");
user.to_tlbx("user", "user.tlbx", true)?;
let json = user.to_tealeaf_json("user")?;
```

## Next Steps

- [Derive Macros](./derive-macros.md) -- `#[derive(ToTeaLeaf, FromTeaLeaf)]`
- [Attributes Reference](./attributes.md) -- all `#[tealeaf(...)]` attributes
- [Builder API](./builder.md) -- programmatic document construction
- [Schemas & Types](./schemas.md) -- working with schemas in Rust
- [Error Handling](./errors.md) -- error types and patterns
