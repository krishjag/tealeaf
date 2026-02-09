# Error Handling

TeaLeaf uses the `thiserror` crate for structured error types.

## Error Types

The main error enum:

| Error Variant | Description |
|---------------|-------------|
| `Io` | File I/O error (wraps `std::io::Error`) |
| `InvalidMagic` | Binary file doesn't start with `TLBX` magic bytes |
| `InvalidVersion` | Unsupported binary format version |
| `InvalidType` | Unknown type code in binary data |
| `InvalidUtf8` | String encoding error |
| `UnexpectedToken` | Parse error -- expected one token, got another |
| `UnexpectedEof` | Premature end of input |
| `UnknownStruct` | `@table` references a struct that hasn't been defined |
| `MissingField` | Required field not provided in data |
| `ParseError` | Generic parse error with message |
| `ValueOutOfRange` | Numeric value exceeds target type range |

## Conversion Errors

The `ConvertError` type is used by `FromTeaLeaf`:

```rust
pub enum ConvertError {
    MissingField { struct_name: String, field: String },
    TypeMismatch { expected: String, got: String, path: String },
    Nested { path: String, source: Box<ConvertError> },
    Custom(String),
}
```

## Handling Errors

### Parse Errors

```rust
use tealeaf::TeaLeaf;

match TeaLeaf::parse(input) {
    Ok(doc) => { /* use doc */ },
    Err(e) => {
        eprintln!("Parse error: {}", e);
        // e.g., "Unexpected token: expected ':', got '}' at line 5"
    }
}
```

### I/O Errors

```rust
match TeaLeaf::load("nonexistent.tl") {
    Ok(doc) => { /* ... */ },
    Err(e) => {
        // Will be an Io variant wrapping std::io::Error
        eprintln!("Could not load file: {}", e);
    }
}
```

### Binary Format Errors

```rust
use tealeaf::Reader;

match Reader::open("corrupted.tlbx") {
    Ok(reader) => { /* ... */ },
    Err(e) => {
        // Could be InvalidMagic, InvalidVersion, etc.
        eprintln!("Binary read error: {}", e);
    }
}
```

### Conversion Errors

```rust
use tealeaf::{FromTeaLeaf, Value};

let value = Value::String("not a number".into());
match i32::from_tealeaf_value(&value) {
    Ok(n) => println!("Got: {}", n),
    Err(e) => {
        // ConvertError::TypeMismatch { expected: "Int", got: "String" }
        eprintln!("Conversion failed: {}", e);
    }
}
```

## Error Propagation

All errors implement `std::error::Error` and `Display`, so they work with `?` and `anyhow`/`eyre`:

```rust
fn process_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let doc = TeaLeaf::load(path)?;
    let json = doc.to_json()?;
    doc.compile("output.tlbx", true)?;
    Ok(())
}
```

## Validation Without Errors

For checking validity without consuming the error:

```rust
let is_valid = TeaLeaf::parse(input).is_ok();
```

The CLI `validate` command uses this pattern to report validity without stopping on errors.
