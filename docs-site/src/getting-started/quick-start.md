# Quick Start

This guide walks through the core TeaLeaf workflow: write text, compile to binary, and convert to/from JSON.

## 1. Write a TeaLeaf File

Create `example.tl`:

```tl
# Define schemas
@struct address (street: string, city: string, zip: string)
@struct user (
  id: int,
  name: string,
  email: string?,
  address: address,
  active: bool,
)

# Data uses schemas -- field names defined once, not repeated
users: @table user [
  (1, "Alice", "alice@example.com", ("123 Main St", "Seattle", "98101"), true),
  (2, "Bob", ~, ("456 Oak Ave", "Austin", "78701"), false),
]

# Plain key-value pairs
app_version: "2.0.0"
debug: false
```

## 2. Validate

Check that the file is syntactically correct:

```bash
tealeaf validate example.tl
```

## 3. Compile to Binary

Compile to the compact binary format:

```bash
tealeaf compile example.tl -o example.tlbx
```

## 4. Inspect

View information about either format:

```bash
tealeaf info example.tl
tealeaf info example.tlbx
```

## 5. Convert to JSON

```bash
# Text to JSON
tealeaf to-json example.tl -o example.json

# Binary to JSON
tealeaf tlbx-to-json example.tlbx -o example_from_binary.json
```

## 6. Convert from JSON

```bash
# JSON to TeaLeaf text (with automatic schema inference)
tealeaf from-json example.json -o reconstructed.tl

# JSON to TeaLeaf binary
tealeaf json-to-tlbx example.json -o direct.tlbx
```

## 7. Decompile

Convert binary back to text:

```bash
tealeaf decompile example.tlbx -o decompiled.tl
```

## Complete Workflow

```
example.tl ──compile──> example.tlbx ──decompile──> decompiled.tl
    │                       │
    ├──to-json──> example.json <──tlbx-to-json──┘
    │                │
    └──from-json─────┘
```

## Using the Rust API

```rust
use tealeaf_core::TeaLeaf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse text format
    let doc = TeaLeaf::load("example.tl")?;

    // Access values
    if let Some(users) = doc.get("users") {
        println!("Users: {:?}", users);
    }

    // Compile to binary
    doc.compile("example.tlbx", true)?;

    // Convert to JSON
    let json = doc.to_json()?;
    println!("{}", json);

    Ok(())
}
```

### With Derive Macros

```rust
use tealeaf_core::{TeaLeaf, ToTeaLeaf, FromTeaLeaf, ToTeaLeafExt};

#[derive(ToTeaLeaf, FromTeaLeaf)]
struct User {
    id: i32,
    name: String,
    #[tealeaf(optional)]
    email: Option<String>,
    active: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = User {
        id: 1,
        name: "Alice".into(),
        email: Some("alice@example.com".into()),
        active: true,
    };

    // Serialize to TeaLeaf text
    let text = user.to_tl_string("user");
    println!("{}", text);

    // Compile directly to binary
    user.to_tlbx("user", "user.tlbx", false)?;

    // Deserialize from a document
    let doc = TeaLeaf::load("user.tlbx")?;
    let loaded = User::from_tealeaf_value(doc.get("user").unwrap())?;

    Ok(())
}
```

## Using the .NET API

### Source Generator (Compile-Time)

```csharp
using TeaLeaf;
using TeaLeaf.Annotations;

[TeaLeaf]
public partial class User
{
    public int Id { get; set; }
    public string Name { get; set; } = "";

    [TLOptional]
    public string? Email { get; set; }

    public bool Active { get; set; }
}

// Serialize
var user = new User { Id = 1, Name = "Alice", Active = true };
string text = user.ToTeaLeafText();
string json = user.ToTeaLeafJson();
user.CompileToTeaLeaf("user.tlbx");

// Deserialize
using var doc = TLDocument.ParseFile("user.tlbx");
var loaded = User.FromTeaLeaf(doc);
```

### Reflection Serializer (Runtime)

```csharp
using TeaLeaf;

var user = new User { Id = 1, Name = "Alice", Active = true };

// Serialize
using var doc = TeaLeafSerializer.ToDocument(user);
string text = TeaLeafSerializer.ToText(user);
string json = TeaLeafSerializer.ToJson(user);

// Deserialize
using var doc2 = TLDocument.ParseFile("user.tlbx");
var loaded = TeaLeafSerializer.Deserialize<User>(doc2);
```

## Next Steps

- [Core Concepts](./concepts.md) -- understand schemas, types, and the text format
- [CLI Reference](../cli/overview.md) -- all available commands
- [Rust Guide](../rust/overview.md) -- Rust API in depth
- [.NET Guide](../dotnet/overview.md) -- .NET bindings in depth
