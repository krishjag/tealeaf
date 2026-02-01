# Changelog

## v2.0.0-beta.1 (Current)

Initial public beta release.

### Format
- Text format (`.tl`) with comments, schemas, and all value types
- Binary format (`.tlbx`) with string deduplication, schema embedding, and per-section compression
- 15 primitive types + 6 container/semantic types
- Inline schemas with `@struct`, `@table`, `@map`, `@union`
- References (`!name`) and tagged values (`:tag value`)
- File includes (`@include`)
- ISO 8601 timestamp support
- JSON bidirectional conversion with schema inference

### CLI
- 8 commands: `compile`, `decompile`, `info`, `validate`, `to-json`, `from-json`, `tlbx-to-json`, `json-to-tlbx`
- Pre-built binaries for 7 platforms (Windows, Linux, macOS -- x64 and ARM64)

### Rust
- `tealeaf-core` crate with full parser, compiler, and reader
- `tealeaf-derive` crate with `#[derive(ToTeaLeaf, FromTeaLeaf)]`
- Builder API (`TeaLeafBuilder`)
- Memory-mapped binary reading
- Conversion traits with automatic schema collection

### .NET
- `TeaLeaf` NuGet package with native libraries for all platforms
- C# incremental source generator (`[TeaLeaf]` attribute)
- Reflection-based serializer (`TeaLeafSerializer`)
- Managed wrappers (`TLDocument`, `TLValue`, `TLReader`)
- Schema introspection API
- Diagnostic codes TL001-TL006

### FFI
- C-compatible API via `tealeaf-ffi` crate
- 45+ exported functions
- Thread-safe error handling
- Null-safe for all pointer parameters
- C header generation via `cbindgen`

### Known Limitations
- `@union` is parsed but not encoded in binary schema table
- Bytes type does not round-trip through text format
- JSON import does not recognize `$ref`, `$tag`, or timestamp strings
- String length limited to 65,535 bytes in binary format
- 64-byte header overhead makes TeaLeaf inefficient for very small objects

---

*TeaLeaf v2.0.0-beta.1 -- Peace between human and machine.*
