# Canonical Test Suite

This directory contains canonical sample files for validating end-to-end toolchain behavior.

## Structure

```
canonical/
├── README.md                    # This file
├── samples/                     # Input .tl text files (14 files)
│   ├── primitives.tl            # Null, bool, int, float, string
│   ├── arrays.tl                # Empty, typed, mixed, nested arrays
│   ├── objects.tl               # Empty, simple, nested, deeply nested objects
│   ├── schemas.tl               # Struct definitions, tables, nested structs
│   ├── special_types.tl         # Refs, tagged values, maps, edge cases
│   ├── timestamps.tl            # ISO 8601 timestamps, timezones
│   ├── numbers_extended.tl      # Hex, binary, scientific notation
│   ├── unions.tl                # Discriminated unions (@union)
│   ├── multiline_strings.tl     # Triple-quoted strings, auto-dedent
│   ├── unicode_escaping.tl      # Unicode ranges, escape sequences
│   ├── refs_tags_maps.tl        # Comprehensive refs, tags, maps tests
│   ├── mixed_schemas.tl         # Schema + schemaless data together
│   ├── large_data.tl            # Stress tests for size limits
│   └── cyclic_refs.tl           # Reference cycles and forward refs
├── expected/                    # Expected JSON outputs
│   └── *.json
├── binary/                      # Pre-compiled .tlbx files
│   └── *.tlbx
└── errors/                      # Invalid input files for error testing
    ├── expected_errors.json     # Expected error messages
    └── *.tl / *.tlbx            # Invalid input files
```

## Test Cases

### primitives.tl
Tests primitive types: `null`, `bool`, `int`, `float`, `string`
- Includes: negative numbers, large integers, Unicode, emoji, escape sequences

### arrays.tl
Tests array constructs: empty arrays, typed arrays, mixed arrays, nested arrays
- Includes: deeply nested arrays (3 levels), arrays of objects

### objects.tl
Tests object constructs: empty objects, simple objects, nested objects
- Includes: deeply nested objects (4 levels), mixed content objects

### schemas.tl
Tests schema-based data: `@struct` definitions, `@table` directive
- Includes: Point, User, Address, Employee structs with nesting and nullable fields

### special_types.tl
Tests special TeaLeaf types: references (`!ref`), tagged values (`:tag`), maps (`@map`)
- Includes: pseudo-references (plain objects with `$ref` key), edge cases

### timestamps.tl
Tests ISO 8601 timestamps in various formats
- Includes: date-only, date+time UTC, milliseconds, timezone offsets (+05:30, -08:00)
- Includes: timestamps in arrays, objects, and @table

### numbers_extended.tl
Tests extended number formats
- Includes: hexadecimal (`0xFF`), binary (`0b1010`), scientific notation (`6.022e23`)
- Includes: large integers near i64 limits, edge cases

### unions.tl
Tests discriminated unions with `@union` directive
- Includes: Shape, Result, Maybe unions with variants
- Includes: empty variants, multi-field variants, nested usage

### multiline_strings.tl
Tests triple-quoted multiline strings
- Includes: auto-dedenting, code blocks, SQL, JSON templates, Markdown
- Includes: in arrays, objects, and edge cases

### unicode_escaping.tl
Tests Unicode characters and escape sequences
- Escape sequences: `\n`, `\t`, `\r`, `\"`, `\\`
- Unicode ranges: CJK (Japanese, Chinese, Korean), European (Greek, Cyrillic), Arabic, Hebrew, Indic
- Emoji: faces, flags, compound emoji (ZWJ), skin tones
- Edge cases: Unicode in keys, combining characters, nested quotes

### refs_tags_maps.tl
Comprehensive tests for references, tagged values, and maps
- References (`!ref`): basic definitions, nested data, simple values, arrays, multiple usage
- Tagged values (`:tag`): various value types, nested tags, Result/Option patterns
- Maps (`@map`): string keys, integer keys, mixed keys, complex values, nested maps
- Combined usage: refs in maps, tagged refs, complex compositions

### mixed_schemas.tl
Tests mixing schema-bound and schemaless data in one file
- Schema definitions: `@struct`, `@union` with various field types
- Schema-bound data: `@table` with single/nested schemas
- Schemaless data: plain objects, arrays, heterogeneous data
- Mixed contexts: schema tables alongside plain objects in same parent
- Edge cases: empty tables, single-row tables, point-like objects without schema

### large_data.tl
Stress tests for large data and size limits
- Huge arrays: 100 integers, 50 strings, 30 objects
- Deep nesting: 10 levels of objects, 10 levels of arrays, mixed nesting
- Long strings: 500 chars, 1000 chars, long multiline, long Unicode
- Large maps: 25 string keys, 25 integer keys, complex values
- Large tables: 20-row schema-bound table
- Combined stress: mega objects with nested arrays, objects, matrices
- Edge cases: empty nested containers, single elements, repeated data (deduplication)

### cyclic_refs.tl
Tests reference cycles and forward references
- Two-node cycle: A → B → A
- Self-reference: A → A (node references itself)
- Three-node cycle: A → B → C → A
- Forward references: use before definition
- Deeply nested cycles: cycles within nested objects
- Refs in various contexts: arrays, objects, tagged values, maps

**Note on Reference Semantics:**
TeaLeaf uses **symbolic/lazy references**. References are stored by name (`Value::Ref("name")`) and are NOT automatically dereferenced. This design:
- Allows cyclic references without infinite recursion
- Enables graph-like data structures
- Leaves resolution to the consuming application
- Exports to JSON as `{"$ref": "ref_name"}`

## Error Message Golden Tests

The `errors/` directory contains invalid input files that must produce specific error messages. These tests ensure error messages remain stable and useful across CLI, FFI, and .NET interfaces.

### Error Cases

| File | Error Type | Description |
|------|------------|-------------|
| `unterminated_string.tl` | ParseError | String without closing quote |
| `unterminated_multiline.tl` | ParseError | Triple-quoted string without closing `"""` |
| `invalid_hex.tl` | ParseError | Hex number with invalid characters |
| `invalid_binary.tl` | ParseError | Binary number with invalid digits |
| `unexpected_token.tl` | UnexpectedToken | Missing colon between key/value |
| `unclosed_brace.tl` | UnexpectedToken | Object without closing `}` |
| `unclosed_bracket.tl` | UnexpectedToken | Array without closing `]` |
| `include_not_found.tl` | ParseError | Include file doesn't exist |
| `invalid_magic.tlbx` | InvalidMagic | Binary file with wrong magic bytes |

### Running Error Tests

```bash
# Run all error tests (10 tests)
cargo test -p tealeaf-core error_

# Run with verbose output
cargo test -p tealeaf-core error_ -- --nocapture
```

### Error Message Contracts

Error messages should:
- Be human-readable and descriptive
- Include context (e.g., what was expected vs. got)
- Remain stable across versions (breaking changes require version bump)
- Work consistently across all interfaces (CLI, FFI, .NET)

## Validation Flow

### 1. Text → JSON
Parse `.tl` text file and export to JSON. Compare with `expected/*.json`.

### 2. Binary → JSON
Read `.tlbx` binary file and export to JSON. Compare with `expected/*.json`.

### 3. Text → Binary → JSON (Full Roundtrip)
Parse `.tl`, compile to temporary `.tlbx`, read back, and export to JSON.

## Running Validation

### Rust Tests

```bash
# Run all canonical tests (52 tests: 42 success + 10 error)
cargo test -p tealeaf-core canonical

# Run with verbose output
cargo test -p tealeaf-core canonical -- --nocapture
```

### CLI Validation

```bash
# Text → JSON
tealeaf to-json canonical/samples/primitives.tl | diff - canonical/expected/primitives.json

# Binary → JSON
tealeaf tlbx-to-json canonical/binary/primitives.tlbx | diff - canonical/expected/primitives.json

# Validate all samples
for f in primitives arrays objects schemas special_types timestamps numbers_extended unions multiline_strings unicode_escaping refs_tags_maps mixed_schemas large_data cyclic_refs; do
  echo -n "$f: "
  tealeaf to-json canonical/samples/${f}.tl | diff -q - canonical/expected/${f}.json && echo "PASS" || echo "FAIL"
done
```

### .NET Tests

```bash
dotnet test --filter "Canonical"
```

## Regenerating Expected Outputs

If you modify the samples or fix bugs in the toolchain:

```bash
# Regenerate expected JSON files
for f in primitives arrays objects schemas special_types timestamps numbers_extended unions multiline_strings unicode_escaping refs_tags_maps mixed_schemas large_data cyclic_refs; do
  tealeaf to-json canonical/samples/${f}.tl -o canonical/expected/${f}.json
done

# Regenerate binary files
for f in primitives arrays objects schemas special_types timestamps numbers_extended unions multiline_strings unicode_escaping refs_tags_maps mixed_schemas large_data cyclic_refs; do
  tealeaf compile canonical/samples/${f}.tl -o canonical/binary/${f}.tlbx
done
```

## Adding New Test Cases

1. Create `samples/new_test.tl` with input data
2. Generate expected output:
   ```bash
   tealeaf to-json samples/new_test.tl -o expected/new_test.json
   tealeaf compile samples/new_test.tl -o binary/new_test.tlbx
   ```
3. Add test functions to `tealeaf-core/tests/canonical.rs`
4. Verify tests pass: `cargo test -p tealeaf-core canonical`
5. Commit all files together

## JSON Conversion Contracts

These contracts define how special TeaLeaf types serialize to JSON:

| TeaLeaf Type | JSON Format                                    |
|--------------|------------------------------------------------|
| Null         | `null`                                         |
| Bool         | `true` / `false`                               |
| Int/UInt     | number                                         |
| Float        | number (NaN/Infinity → `null`)                 |
| String       | string                                         |
| Bytes        | `"0xdeadbeef"` (lowercase hex with 0x prefix)  |
| Timestamp    | `"2024-01-15T10:30:00.123Z"` (ISO 8601 UTC)    |
| Array        | array                                          |
| Object       | object                                         |
| Map          | `[[key1, val1], [key2, val2], ...]`            |
| Ref          | `{"$ref": "ref_name"}`                         |
| Tagged       | `{"$tag": "tag_name", "$value": <value>}`      |
