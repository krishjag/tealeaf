# Changelog

## v2.0.0-beta.6 (Current)

### Features
- **Recursive array schema inference in JSON import** — `from_json_with_schemas` now discovers schemas for arrays nested inside objects at arbitrary depth (e.g., `items[].product.stock[]`). Previously, `analyze_nested_objects` only recursed into nested objects but not nested arrays, causing deeply nested arrays to fall back to `[]any`. The CLI and derive-macro paths now produce equivalent schema coverage.

### Tooling
- Version sync scripts (`sync-version.ps1`, `sync-version.sh`) now regenerate the workflow diagram (`assets/tealeaf_workflow.png`) via `generate_workflow_diagram.py` on each version bump

### Testing
- Added `json_inference_nested_array_inside_object` — verifies arrays nested inside objects (e.g., `items[].product.stock[]`) get their own schema and typed array fields
- Verified all 7 fuzz targets pass (~566K total runs, zero crashes)

---

## v2.0.0-beta.5

### Features
- **Schema-aware serialization for Builder API** — `to_tl_with_schemas()` now produces compact `@table` output for documents built via `TeaLeafBuilder` with derive-macro schemas. Previously, PascalCase schema names from `#[derive(ToTeaLeaf)]` (e.g., `SalesOrder`) didn't match the serializer's `singularize()` heuristic (e.g., `"orders"` → `"order"`), causing all arrays to fall back to verbose `[{k: v}]` format. The serializer now resolves schemas via a 4-step chain: declared type from parent schema → singularize → case-insensitive singularize → structural field matching.

### Bug Fixes
- Fixed schema inference name collision when a field singularizes to the same name as its parent array's schema — prevented self-referencing schemas (e.g., `@struct root (root: root)`) and data loss during round-trip (found via fuzzing)
- Fixed `@table` serializer applying wrong schema when the same field name appears at multiple nesting levels with different object shapes — serializer now validates schema fields match the actual object keys before using positional tuple encoding

### Testing
- Added 8 Rust regression tests for schema name collisions: `fuzz_repro_dots_in_field_name`, `schema_name_collision_field_matches_parent`, `analyze_node_nesting_stress_test`, `schema_collision_recursive_arrays`, `schema_collision_recursive_same_shape`, `schema_collision_three_level_nesting`, `schema_collision_three_level_divergent_leaves`, `all_orders_cli_vs_api_roundtrip`
- Added derive integration test `test_builder_schema_aware_table_output` — verifies Builder API with 5 nested PascalCase schemas produces `@table` encoding and round-trips correctly
- Verified all 7 fuzz targets pass (~445K total runs, zero crashes)

---

## v2.0.0-beta.4

### Bug Fixes
- Fixed binary encoding crash when compiling JSON with heterogeneous nested objects — `from_json_with_schemas` infers `any` pseudo-type for fields whose nested objects have varying shapes; the binary encoder now falls back to generic encoding instead of erroring with "schema-typed field 'any' requires a schema"
- Fixed parser failing to resolve schema names that shadow built-in type keywords — schemas named `bool`, `int`, `string`, etc. now correctly resolve via LParen lookahead disambiguation (struct tuples always start with `(`, primitives never do)
- Fixed `singularize()` producing empty string for single-character field names (e.g., `"s"` → `""`) — caused `@struct` definitions with missing names and unparseable TL text output
- Fixed `validate_tokens.py` token comparison by converting API input to `int` for safety

### .NET
- Added `TLValueExtensions` with `GetRequired()` extension methods for `TLValue` and `TLDocument` — provides non-nullable access patterns, reducing CS8602 warnings in consuming code
- Added TL007 diagnostic: `[TeaLeaf]` classes in the global namespace now produce a compile-time error ("TeaLeaf type must be in a named namespace")
- Removed `SuppressDependenciesWhenPacking` property from `TeaLeaf.Generators.csproj`
- Exposed `InternalsVisibleTo` for `TeaLeaf.Tests`

### CI/CD
- Re-enabled all 6 GitHub Actions workflows after making the repository public (rust-cli, dotnet-package, accuracy-benchmark, docs, coverage, fuzz)
- Fixed coverlet filter quoting in coverage workflow — commas URL-encoded as `%2c` to prevent shell argument splitting
- Fixed Codecov token handling — made `CODECOV_TOKEN` optional for public repo tokenless uploads
- Fixed Codecov multi-file upload format — changed from YAML block scalar to comma-separated single-line
- Refactored coverage workflow to use `dotnet-coverage` with dedicated settings XML files
- Added CodeQL security analysis workflow
- Fixed accuracy-benchmark workflow permissions

### Testing
- Added Rust regression test for `any` pseudo-type compile round-trip
- Added 21 Rust tests for schema names shadowing all built-in type keywords (`bool`, `int`, `int8`..`int64`, `uint`..`uint64`, `float`, `float32`, `float64`, `string`, `timestamp`, `bytes`) — covers JSON inference round-trip, direct TL parsing, self-referencing schemas, duplicate declarations, and multiple built-in-named schemas in one document
- Added 4 .NET regression tests covering `TLDocument.FromJson` → `Compile` with heterogeneous nested objects, mixed-structure arrays, complex schema inference, and retail_orders.json end-to-end
- Added .NET tests for JSON serialization of timestamps and byte arrays
- Added .NET coverage tests for multi-word enums and nullable nested objects
- Added .NET source generator tests (524 new lines in `GeneratorTests.cs`) including TL007 global namespace diagnostic
- Added .NET `TLValue.GetRequired()` extension method tests
- Added .NET `TLReader` binary reader tests (168 new lines)
- Added cross-platform `FindRepoFile` helper for .NET test fixture discovery (walks up directory tree instead of hardcoded relative path depth)
- Verified full .NET test suite on Linux (WSL Ubuntu 24.04)

### Tooling
- Added `--version` / `-V` CLI flag
- Added `delete-caches.ps1` and `delete-caches.sh` GitHub Actions cache cleanup scripts
- Updated `coverage.ps1` to support `dotnet-coverage` collection with XML settings files

### Documentation
- Updated binary deserialization method names in quick-start, LLM context guide, schema evolution guide, and derive macros docs
- Updated tealeaf workflow diagram

---

## v2.0.0-beta.3

### Features
- **Byte literals** — `b"..."` hex syntax for byte data in text format (e.g., `payload: b"cafef00d"`)
- **Arbitrary-precision numbers** — `Value::JsonNumber` preserves exact decimal representation for numbers exceeding native type ranges
- **Insertion order preservation** — `IndexMap` replaces `HashMap` for all user-facing containers; JSON round-trips now preserve original key order ([ADR-0001](../decisions/adr-0001-indexmap-insertion-order.md))
- **Timestamp timezone support** — Timestamps encode timezone offset in minutes (10 bytes: 8 millis + 2 offset); supports `Z`, `+HH:MM`, `-HH:MM`, `+HH` formats
- **Special float values** — `NaN`, `inf`, `-inf` keywords for IEEE 754 special values (JSON export converts to `null`)
- **Extended escape sequences** — `\b` (backspace), `\f` (form feed), `\uXXXX` (Unicode code points) for full JSON string escape parity
- **Forward compatibility** — Unknown directives silently ignored, enabling older implementations to partially parse files with newer features (spec §1.18)

### Bug Fixes
- Fixed bounds check failures and bitmap overflow issues in binary decoder
- Fixed lexer infinite loop on certain malformed inputs (found via fuzzing)
- Fixed NaN value quoting causing incorrect round-trip behavior
- Fixed parser crashes on deeply nested structures
- Fixed integer overflow in varint decoding
- Fixed off-by-one errors in array length checks
- Fixed negative hex/binary literal parsing
- Fixed exponent-only numbers (e.g., `1e3`) to parse as floats, not integers
- Fixed timestamp timezone parsing to accept hour-only offsets (`+05` = `+05:00`)
- Rejected value-only types (`object`, `map`, `tuple`, `ref`, `tagged`) as schema field types per spec §2.1
- Fixed .NET package publishing for `TeaLeaf.Annotations` and `TeaLeaf.Generators` to NuGet

### Performance
- Removed O(n log n) key sorting from all serialization paths: 6-17% faster for small/medium objects, up to 69% faster for tabular data
- Binary decode 56-105% slower for generic object workloads due to `IndexMap` insertion cost (acceptable trade-off per ADR-0001; columnar workloads less affected)

### Specification
- Schema table header byte +6 stores Union Count (was reserved)
- String table length encoding changed from `u16` to `u32` for strings > 65KB
- Added type code `0x12` for `JSONNUMBER`
- Timestamp encoding extended to 10 bytes (8 millis + 2 offset)
- Added `bytes_lit` grammar production; extended `number` to include `NaN`/`inf`/`-inf`
- Documented `object`, `map`, `ref`, `tagged` as value-only types (not valid in schema fields)
- Resolved compression algorithm spec contradiction: binary format v2 uses ZLIB (deflate), not zstd ([ADR-0004](../decisions/adr-0004-zlib-compression.md))

### Tooling
- **Fuzzing infrastructure** — 7 cargo-fuzz targets with custom dictionaries and structure-aware generation ([ADR-0002](../decisions/adr-0002-fuzzing-architecture.md))
- **Fuzzing CI workflow** — GitHub Actions runs all targets for 120s each (~15 min per run)
- **Nesting depth limit** — 256-level max for stack overflow protection ([ADR-0003](../decisions/adr-0003-nesting-depth-limit.md))
- **VS Code extension** — Syntax highlighting for `.tl` files (`vscode-tealeaf/`)
- **FFI safety** — Comprehensive `# Safety` docs on all FFI functions; regenerated `tealeaf.h`
- **Token validation** — `validate_tokens.py` script validates API-reported token counts against tiktoken
- **Maintenance scripts** — `delete-deployments` and `delete-workflow-runs` for GitHub cleanup

### Testing
- 238+ adversarial tests for malformed binary input
- 333+ .NET edge case tests for FFI boundary conditions
- Property-based tests with depth-bounded recursive generation
- Accuracy benchmark token savings updated to **~36% fewer data tokens** (validated with tiktoken)

### Documentation
- ADR-0001: IndexMap for Insertion Order Preservation
- ADR-0002: Fuzzing Architecture and Strategy
- ADR-0003: Maximum Nesting Depth Limit (256)
- ADR-0004: ZLIB Compression for Binary Format
- Code of Conduct, SECURITY.md, GitHub issue/PR templates
- `examples/showcase.tl` — 736-line comprehensive format demonstration
- Sample accuracy benchmark results

### Breaking Changes
- `Value::Object` uses `IndexMap<String, Value>` instead of `HashMap` (type alias `ObjectMap` provided; `From<HashMap>` retained for backward compatibility)
- `Value::Timestamp(i64)` → `Value::Timestamp(i64, i16)` — second field is timezone offset in minutes
- `Value::JsonNumber(String)` variant added — match expressions on `Value` need new arm
- Binary timestamps not backward-compatible (beta.2 readers cannot decode beta.3 timestamps; beta.3 readers handle beta.2 files by defaulting offset to UTC)
- JSON round-trips preserve key order instead of alphabetizing

---

## v2.0.0-beta.2

### Format
- `@union` definitions now encoded in binary schema table (full text-binary-text roundtrip)
- Union schema region uses backward-compatible extension of schema table header
- Derive macro `collect_unions()` generates union definitions for Rust enums
- `TeaLeafBuilder::add_union()` for programmatic union construction

### Improvements
- Version sync automation expanded to cover all project files (16 targets)
- NuGet package icon added to all NuGet packages (TeaLeaf, Annotations, Generators)
- CI badges added to README (Rust CI, .NET CI, crates.io, NuGet, codecov, License)
- crates.io publish ordering fixed (`tealeaf-derive` before `tealeaf-core`)
- Contributing guide added (`CONTRIBUTING.md`)
- Spec governance documentation added
- Accuracy benchmark `dump-prompts` subcommand for offline prompt inspection
- `TeaLeaf.Annotations` published as separate NuGet package (fixes dependency resolution)
- `benches_proto/` excluded from crates.io package (removes `protoc` requirement for consumers)

---

## v2.0.0-beta.1

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
- ~~Bytes type does not round-trip through text format~~ (resolved: `b"..."` hex literals added)
- JSON import does not recognize `$ref`, `$tag`, or timestamp strings
- Individual string length limited to ~4 GB (u32) in binary format
- 64-byte header overhead makes TeaLeaf inefficient for very small objects

---

