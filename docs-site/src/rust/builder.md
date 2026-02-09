# Builder API

The `TeaLeafBuilder` provides a fluent API for constructing TeaLeaf documents programmatically.

## Basic Usage

```rust
use tealeaf::{TeaLeafBuilder, Value};

let doc = TeaLeafBuilder::new()
    .add_value("name", Value::String("Alice".into()))
    .add_value("age", Value::Int(30))
    .add_value("active", Value::Bool(true))
    .build();

// Compile to binary
doc.compile("output.tlbx", true)?;

// Convert to JSON
let json = doc.to_json()?;
```

## Methods

### `new()`

Create a new empty builder:

```rust
let builder = TeaLeafBuilder::new();
```

### `add_value(key, value)`

Add a raw `Value` to the document:

```rust
builder.add_value("count", Value::Int(42))
```

### `add<T: ToTeaLeaf>(key, dto)`

Add a struct that implements `ToTeaLeaf`. Automatically collects schemas from the type:

```rust
#[derive(ToTeaLeaf)]
struct Config {
    host: String,
    port: i32,
}

let config = Config { host: "localhost".into(), port: 8080 };

let doc = TeaLeafBuilder::new()
    .add("config", &config)
    .build();
```

### `add_vec<T: ToTeaLeaf>(key, items)`

Add an array of `ToTeaLeaf` items. Automatically collects schemas:

```rust
let users = vec![
    User { id: 1, name: "Alice".into() },
    User { id: 2, name: "Bob".into() },
];

let doc = TeaLeafBuilder::new()
    .add_vec("users", &users)
    .build();
```

### `add_schema(schema)`

Manually add a schema definition:

```rust
use tealeaf::{Schema, Field, FieldType};

let schema = Schema {
    name: "point".to_string(),
    fields: vec![
        Field {
            name: "x".into(),
            field_type: FieldType { base: "int".into(), nullable: false, is_array: false },
        },
        Field {
            name: "y".into(),
            field_type: FieldType { base: "int".into(), nullable: false, is_array: false },
        },
    ],
};

let doc = TeaLeafBuilder::new()
    .add_schema(schema)
    .add_value("origin", Value::Array(vec![Value::Int(0), Value::Int(0)]))
    .build();
```

### `root_array()`

Mark the document as a root-level array (rather than a key-value document):

```rust
let doc = TeaLeafBuilder::new()
    .root_array()
    .add_value("items", Value::Array(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ]))
    .build();
```

### `build()`

Finalize and return the `TeaLeaf` document:

```rust
let doc = builder.build();
```

## Complete Example

```rust
use tealeaf::{TeaLeafBuilder, ToTeaLeaf, FromTeaLeaf, Value};

#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Address {
    street: String,
    city: String,
}

#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Employee {
    id: i64,
    name: String,
    address: Address,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let employees = vec![
        Employee {
            id: 1,
            name: "Alice".into(),
            address: Address { street: "123 Main".into(), city: "Seattle".into() },
        },
        Employee {
            id: 2,
            name: "Bob".into(),
            address: Address { street: "456 Oak".into(), city: "Austin".into() },
        },
    ];

    let doc = TeaLeafBuilder::new()
        .add_value("company", Value::String("Acme Corp".into()))
        .add_vec("employees", &employees)
        .add_value("version", Value::Int(1))
        .build();

    // Output
    doc.compile("company.tlbx", true)?;
    println!("{}", doc.to_tl_with_schemas());
    println!("{}", doc.to_json()?);

    Ok(())
}
```
