# Architecture

High-level architecture of the TeaLeaf project.

## Crate Structure

```
tealeaf/
├── tealeaf-core/          # Core library + CLI
│   ├── src/
│   │   ├── main.rs        # CLI entry point
│   │   ├── lib.rs         # Public API (TeaLeaf, Value, Schema, traits)
│   │   ├── reader.rs      # Binary file reader
│   │   ├── writer.rs      # Binary file writer (compiler)
│   │   ├── builder.rs     # TeaLeafBuilder fluent API
│   │   └── convert.rs     # ToTeaLeaf/FromTeaLeaf trait impls for primitives
│   └── tests/
│       ├── canonical.rs   # Canonical fixture tests
│       └── derive.rs      # Derive macro tests
│
├── tealeaf-derive/        # Proc-macro crate
│   ├── lib.rs             # Macro entry points
│   ├── attrs.rs           # Attribute parsing
│   ├── to_tealeaf.rs      # ToTeaLeaf derive implementation
│   ├── from_tealeaf.rs    # FromTeaLeaf derive implementation
│   ├── schema.rs          # Schema generation logic
│   └── util.rs            # Shared utilities
│
├── tealeaf-ffi/           # C FFI layer
│   ├── src/lib.rs         # All FFI exports
│   └── build.rs           # cbindgen header generation
│
├── bindings/dotnet/       # .NET bindings
│   ├── TeaLeaf.Annotations/   # Attribute definitions
│   ├── TeaLeaf.Generators/    # Source generator
│   ├── TeaLeaf/               # Managed wrappers + serializer
│   └── TeaLeaf.Tests/         # Test project
│
├── canonical/             # Canonical test fixtures
│   ├── samples/           # .tl text files
│   ├── expected/          # Expected .json outputs
│   ├── binary/            # Pre-compiled .tlbx files
│   └── errors/            # Invalid files for error testing
│
└── spec/                  # Format specification
    └── TEALEAF_SPEC.md
```

## Data Flow

### Parse Pipeline

```
Text input (.tl)
    │
    ▼
Lexer → Token stream
    │
    ▼
Parser → AST (directives + key-value pairs)
    │
    ├── Schema definitions → IndexMap<String, Schema>
    ├── Reference definitions → resolved inline
    └── Key-value pairs → IndexMap<String, Value>
    │
    ▼
TeaLeaf { schemas, data }
```

### Compile Pipeline

```
TeaLeaf { schemas, data }
    │
    ▼
String collector → String table (deduplicated)
    │
    ▼
Schema encoder → Schema table (binary)
    │
    ▼
Value encoder → Data sections (per key)
    │    │
    │    ├── Primitives → fixed-size encoding
    │    ├── Strings → string table index (u32)
    │    ├── Struct arrays → null bitmap + positional values
    │    └── Other → type-tagged encoding
    │
    ▼
Compressor (per section, if > 64 bytes)
    │
    ▼
Writer → .tlbx file
    ├── Header (64 bytes)
    ├── String table
    ├── Schema table
    ├── Section index
    └── Data sections
```

### Read Pipeline

```
.tlbx file
    │
    ▼
Reader (or MmapReader)
    │
    ├── Header validation (magic, version)
    ├── String table → lazy access
    ├── Schema table → lazy access
    └── Section index → key → offset mapping
    │
    ▼
Value access (by key)
    │
    ├── Locate section in index
    ├── Decompress if needed
    ├── Decode value by type code
    └── Return Value enum
```

## Key Design Decisions

### Positional Schema Encoding

Field names appear only in the schema table. Data rows use position to identify fields. This trades readability of binary for compactness.

### Per-Section Compression

Each top-level key is a separate section compressed independently. This allows:
- Random access without decompressing the entire file
- Selective decompression (only read sections you need)

### Thread-Local Error Handling (FFI)

The FFI uses thread-local storage for error messages instead of out-parameters or exceptions. This simplifies the C API while remaining thread-safe.

### Source Generator vs Reflection

The .NET binding offers both approaches because:
- Source generators produce optimal code but require `partial` classes
- Reflection works with any type but is slower
- Both share the same native library for actual encoding/decoding

### Insertion Order Preservation (IndexMap)

All user-facing maps use `IndexMap` instead of `HashMap` to preserve insertion order across format conversions. Internal lookup tables (string interning, schema/union resolution, caches) remain `HashMap` for performance. See [ADR-0001](../decisions/adr-0001-indexmap-insertion-order.md) for the full decision record including benchmark impact.

### No Schema Versioning

TeaLeaf deliberately avoids schema evolution machinery. The rationale:
- Simpler implementation and specification
- Source file is always the truth
- Recompilation is explicit and deterministic
- Applications that need evolution can layer it on top
