# Schemas & Types

Working with schemas and the type system in Rust.

## Schema Structure

```rust
pub struct Schema {
    pub name: String,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub name: String,
    pub field_type: FieldType,
}

pub struct FieldType {
    pub base: String,       // "int", "string", "user", etc.
    pub nullable: bool,     // field: T?
    pub is_array: bool,     // field: []T
}
```

## Creating Schemas Manually

```rust
use tealeaf::{Schema, Field, FieldType};

let user_schema = Schema {
    name: "user".to_string(),
    fields: vec![
        Field {
            name: "id".into(),
            field_type: FieldType { base: "int".into(), nullable: false, is_array: false },
        },
        Field {
            name: "name".into(),
            field_type: FieldType { base: "string".into(), nullable: false, is_array: false },
        },
        Field {
            name: "tags".into(),
            field_type: FieldType { base: "string".into(), nullable: false, is_array: true },
        },
        Field {
            name: "email".into(),
            field_type: FieldType { base: "string".into(), nullable: true, is_array: false },
        },
    ],
};
```

## Collecting Schemas from Derive

When using `#[derive(ToTeaLeaf)]`, schemas are collected automatically:

```rust
#[derive(ToTeaLeaf)]
struct Address { street: String, city: String }

#[derive(ToTeaLeaf)]
struct User { name: String, home: Address }

// Collects schemas for both `user` and `address`
let schemas = User::collect_schemas();
assert!(schemas.contains_key("user"));
assert!(schemas.contains_key("address"));
```

## Accessing Schemas from Documents

```rust
let doc = TeaLeaf::load("data.tl")?;

// Get a specific schema
if let Some(schema) = doc.schema("user") {
    println!("Schema: {} ({} fields)", schema.name, schema.fields.len());
    for field in &schema.fields {
        let nullable = if field.field_type.nullable { "?" } else { "" };
        let array = if field.field_type.is_array { "[]" } else { "" };
        println!("  {}: {}{}{}", field.name, array, field.field_type.base, nullable);
    }
}

// Iterate all schemas
for (name, schema) in &doc.schemas {
    println!("{}: {} fields", name, schema.fields.len());
}
```

## Accessing Schemas from Binary Reader

Schemas are embedded in the binary format. Parse a key's value and inspect the document schemas:

```rust
use tealeaf::Reader;

let reader = Reader::open("data.tlbx")?;

// List available keys
for key in reader.keys() {
    let value = reader.get(key)?;
    println!("{}: {:?}", key, value);
}
```

For full schema introspection, decompile the binary back to a `TeaLeaf` document and access `doc.schemas`.

## Value Type System

The `Value` enum maps to TeaLeaf types:

| Variant | TeaLeaf Type | Notes |
|---------|-------------|-------|
| `Value::Null` | null | `~` in text |
| `Value::Bool(b)` | bool | |
| `Value::Int(i)` | int/int8/int16/int32/int64 | Size chosen by inference |
| `Value::UInt(u)` | uint/uint8/uint16/uint32/uint64 | Size chosen by inference |
| `Value::Float(f)` | float/float64 | Always f64 at runtime |
| `Value::String(s)` | string | |
| `Value::Bytes(b)` | bytes | |
| `Value::Array(v)` | array | Heterogeneous or typed |
| `Value::Object(m)` | object | String-keyed map |
| `Value::Map(pairs)` | map | Ordered, any key type |
| `Value::Ref(name)` | ref | `!name` reference |
| `Value::Tagged(tag, val)` | tagged | `:tag value` |
| `Value::Timestamp(ms, tz)` | timestamp | Unix milliseconds + timezone offset (minutes) |
| `Value::JsonNumber(s)` | json-number | Arbitrary-precision number (raw JSON decimal string) |

## Type Inference at Write Time

When compiling, the writer selects the smallest encoding:

```rust
// Value::Int(42) → int8 in binary (fits in i8)
// Value::Int(1000) → int16 (fits in i16)
// Value::Int(100_000) → int32 (fits in i32)
// Value::Int(5_000_000_000) → int64
```

## Schema-Typed Data

When data matches a schema (via `@table`), binary encoding uses:
- Positional storage (no field name repetition)
- Null bitmaps (one bit per nullable field)
- Type-homogeneous arrays (packed encoding for `[]int`, `[]string`, etc.)
