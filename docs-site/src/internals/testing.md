# Testing

TeaLeaf has a comprehensive test suite spanning the Rust core, FFI layer, and .NET bindings.

## Test Structure

```
tealeaf/
├── tealeaf-core/tests/
│   ├── canonical.rs          # Canonical fixture round-trip tests
│   └── derive.rs             # Derive macro tests
│
├── tealeaf-ffi/src/lib.rs    # FFI safety tests (inline #[cfg(test)])
│
├── bindings/dotnet/
│   ├── TeaLeaf.Tests/        # .NET unit tests
│   └── TeaLeaf.Generators.Tests/  # Source generator tests
│
└── canonical/                # Shared test fixtures
    ├── samples/              # .tl text files (14 canonical samples)
    ├── expected/             # Expected .json outputs
    ├── binary/               # Pre-compiled .tlbx files
    └── errors/               # Invalid files for error testing
```

## Running Tests

### Rust

```bash
# All Rust tests
cargo test --workspace

# Core tests only
cargo test --package tealeaf-core

# Derive macro tests
cargo test --package tealeaf-core --test derive

# Canonical fixture tests
cargo test --package tealeaf-core --test canonical

# FFI tests
cargo test --package tealeaf-ffi
```

### .NET

```bash
cd bindings/dotnet
dotnet test
```

### Everything

```bash
# Rust
cargo test --workspace

# .NET
cd bindings/dotnet && dotnet test
```

## Canonical Test Fixtures

The `canonical/` directory contains 14 sample files that test every feature:

| Sample | Features Tested |
|--------|----------------|
| `primitives` | All primitive types (bool, int, float, string, null) |
| `arrays` | Simple and nested arrays |
| `objects` | Nested objects |
| `schemas` | `@struct` definitions and `@table` usage |
| `nested_schemas` | Struct-referencing-struct |
| `deep_nesting` | Multi-level struct nesting |
| `nullable` | Nullable fields with `~` values |
| `maps` | `@map` with various key types |
| `references` | `!ref` definitions and usage |
| `tagged` | `:tag value` tagged values |
| `timestamps` | ISO 8601 timestamp parsing |
| `mixed` | Combination of multiple features |
| `comments` | Comment handling |
| `strings` | Quoted, unquoted, multiline strings |

Each sample has:
- `canonical/samples/{name}.tl` — the text source
- `canonical/expected/{name}.json` — expected JSON output
- `canonical/binary/{name}.tlbx` — pre-compiled binary

### Canonical Test Pattern

```rust
#[test]
fn test_canonical_sample() {
    let tl = TeaLeaf::load("canonical/samples/primitives.tl").unwrap();

    // Round-trip: text → binary → text
    let tmp = tempfile::NamedTempFile::new().unwrap();
    tl.compile(tmp.path(), true).unwrap();
    let reader = Reader::open(tmp.path()).unwrap();

    // Verify values match
    assert_eq!(reader.get("count").unwrap().as_int(), Some(42));

    // JSON output matches expected
    let json = tl.to_json().unwrap();
    let expected = std::fs::read_to_string("canonical/expected/primitives.json").unwrap();
    assert_json_eq(&json, &expected);
}
```

## Error Fixtures

The `canonical/errors/` directory contains intentionally invalid files:

| File | Error Tested |
|------|-------------|
| Invalid syntax | Parser error handling |
| Missing struct | `@table` references undefined schema |
| Type mismatches | Schema validation |
| Malformed binary | Binary reader error handling |

## Derive Macro Tests

Tests for `#[derive(ToTeaLeaf, FromTeaLeaf)]`:

- Basic struct serialization/deserialization
- All attribute combinations (`rename`, `skip`, `optional`, `type`, `flatten`, `default`)
- Nested structs
- Enum variants
- Collection types (`Vec`, `HashMap`, `Option`)
- Edge cases (empty structs, single-field structs)

## .NET Test Categories

The .NET test suite covers:

### Source Generator Tests
- Schema generation for all type combinations
- Serialization output (text, JSON, binary)
- Deserialization from documents
- Nested types and collections
- Enum handling
- Attribute processing

### Reflection Serializer Tests
- Generic serialization/deserialization
- Type mapping accuracy
- Nullable handling
- Dictionary and List support

### Native Type Tests
- `TLDocument` lifecycle (parse, access, dispose)
- `TLValue` type accessors
- `TLReader` binary access
- Schema introspection
- Error handling (disposed objects, missing keys)

### DTO Serialization Tests
- Full round-trip (C# object → TeaLeaf → C# object)
- Edge cases (empty strings, nulls, large numbers)
- Collection serialization

## Test Philosophy

1. **Canonical fixtures** — shared across Rust and .NET, ensuring format consistency
2. **Round-trip testing** — text → binary → text verifies no data loss
3. **JSON equivalence** — text → JSON and binary → JSON produce identical output
4. **Error coverage** — every error path has at least one test
5. **Cross-language** — same fixtures tested in Rust, .NET, and via FFI
