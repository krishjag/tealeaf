# tealeaf-core

Schema-aware data format with human-readable text and compact binary representation.

**~43% fewer input tokens than JSON for LLM applications, with zero accuracy loss.**

## Features

- **Dual formats**: Human-readable text (`.tl`) and compact binary (`.tlbx`)
- **Inline schemas**: `@struct`, `@table`, `@map`, `@union` for compact positional data
- **JSON interop**: Bidirectional JSON conversion with automatic schema inference
- **Binary format**: String deduplication, schema embedding, per-section compression
- **CLI**: `compile`, `decompile`, `validate`, `to-json`, `from-json`, and more
- **Derive macros**: `#[derive(ToTeaLeaf, FromTeaLeaf)]` for DTO conversion (with `derive` feature)

## Quick Start

```toml
[dependencies]
tealeaf-core = "2.0.0-beta.9"
```

### Parse and convert

```rust
use tealeaf::TeaLeaf;

// Parse TeaLeaf text
let doc = TeaLeaf::parse("name: \"Alice\"\nage: 30")?;

// Convert to JSON
let json = doc.to_json()?;

// Parse from JSON
let doc = TeaLeaf::from_json(r#"{"name": "Alice", "age": 30}"#)?;

// Get the TeaLeaf text (with inferred schemas)
let text = doc.to_tl_with_schemas();

// Compact output (fewer tokens for LLM context)
use tealeaf::FormatOptions;
let opts = FormatOptions::compact().with_compact_floats();
let compact = doc.to_tl_with_options(&opts);
```

### Derive macros

Enable the `derive` feature for DTO conversion:

```toml
[dependencies]
tealeaf-core = { version = "2.0.0-beta.9", features = ["derive"] }
```

```rust
use tealeaf::{ToTeaLeaf, FromTeaLeaf, ToTeaLeafExt};

#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Employee {
    name: String,
    age: u32,
    department: String,
}

let emp = Employee {
    name: "Alice".into(),
    age: 30,
    department: "Engineering".into(),
};

// Convert to a TeaLeaf document and get text output
let doc = emp.to_tealeaf_doc("employee");
let text = doc.to_tl_with_schemas();
```

## Format Example

JSON input:
```json
{
  "users": [
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25}
  ]
}
```

TeaLeaf text output (with auto-inferred schema):
```
@struct User (name: string, age: int)

users: @table User [
  ("Alice", 30)
  ("Bob", 25)
]
```

## License

MIT

## Links

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [Documentation](https://krishjag.github.io/tealeaf/)
- [Rust Guide](https://krishjag.github.io/tealeaf/rust/overview.html)
- [Format Specification](https://github.com/krishjag/tealeaf/blob/main/spec/TEALEAF_SPEC.md)
