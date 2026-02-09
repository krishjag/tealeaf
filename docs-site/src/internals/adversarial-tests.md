# Adversarial Tests

The adversarial test suite validates TeaLeaf's error handling and robustness using crafted malformed inputs, binary corruption, compression edge cases, and large-corpus stress tests. All tests are isolated in the `adversarial-tests/` directory to avoid touching core project files.

**Current count: 58 tests** across 9 categories.

## Running Tests

```bash
# Run all adversarial tests
cd adversarial-tests/core-harness
cargo test --test adversarial

# With output
cargo test --test adversarial -- --nocapture

# Run via script (PowerShell)
./adversarial-tests/scripts/run_core_harness.ps1

# CLI adversarial tests
./adversarial-tests/scripts/run_cli_adversarial.ps1

# .NET adversarial harness
./adversarial-tests/scripts/run_dotnet_harness.ps1
```

## Test Input Files

### TeaLeaf Format (.tl) — 13 files

Crafted `.tl` files testing parser error paths:

| File | Error Tested | Expected |
|------|-------------|----------|
| `bad_unclosed_string.tl` | Unclosed string literal (`"Alice`) | Parse error |
| `bad_missing_colon.tl` | Missing colon in key-value pair | Parse error |
| `bad_invalid_escape.tl` | Invalid escape sequence (`\q`) | Parse error |
| `bad_number_overflow.tl` | Number exceeding u64 bounds | See note below |
| `bad_table_wrong_arity.tl` | Table row with wrong field count | Parse error |
| `bad_schema_unclosed.tl` | Unclosed `@struct` definition | Parse error |
| `bad_unicode_escape_short.tl` | Incomplete `\u` escape (`\u12`) | Parse error |
| `bad_unicode_escape_invalid_hex.tl` | Invalid hex in `\uZZZZ` | Parse error |
| `bad_unicode_escape_surrogate.tl` | Unicode surrogate pair (`\uD800`) | Parse error |
| `bad_unterminated_multiline.tl` | Unterminated `"""` multiline string | Parse error |
| `invalid_utf8.tl` | Invalid UTF-8 byte sequence | Parse error |

> **Note:** `bad_number_overflow.tl` does not cause a parse error. Numbers exceeding i64/u64 range are stored as `Value::JsonNumber` (exact decimal string), not rejected.

Edge cases that should succeed:

| File | What It Tests | Expected |
|------|--------------|----------|
| `deep_nesting.tl` | 7 levels of nested arrays (`[[[[[[[1]]]]]]]`) | Parse OK |
| `empty_doc.tl` | Empty document | Parse OK |

### JSON Format (.json) — 6 files

Files testing `from_json` error and edge-case paths:

| File | What It Tests | Expected |
|------|--------------|----------|
| `invalid_json_trailing.json` | Trailing comma or content | Parse error |
| `invalid_json_unclosed.json` | Unclosed object or array | Parse error |
| `large_number.json` | Number overflowing f64 | Stored as `JsonNumber` |
| `deep_array.json` | Deeply nested arrays | Parse OK |
| `empty_object.json` | Empty JSON object `{}` | Parse OK |
| `root_array.json` | Root-level array `[1,2,3]` | Preserved as array |

### Binary Format (.tlbx) — 4 files (unused)

These fixture files exist but are **not referenced by any test**. All binary adversarial tests generate malformed data inline using `tempfile::tempdir()`. These files are only used by the CLI adversarial scripts in `results/cli/`.

| File | Content |
|------|---------|
| `bad_magic.tlbx` | Invalid magic bytes |
| `bad_version.tlbx` | Invalid version field |
| `random_garbage.tlbx` | Random bytes |
| `truncated_header.tlbx` | Incomplete header |

## Test Functions

### Parse Error Tests (10 tests)

| Function | Input | Assertion |
|----------|-------|-----------|
| `parse_invalid_syntax_unclosed_string` | `name: "Alice` | `is_err()` |
| `parse_invalid_escape_sequence` | `name: "Alice\q"` | `is_err()` |
| `parse_missing_colon` | `name "Alice"` | `is_err()` |
| `parse_schema_unclosed` | Unclosed `@struct` | `is_err()` |
| `parse_table_wrong_arity` | 3 fields for 2-field schema | `is_err()` |
| `parse_unicode_escape_short` | `\u12` | `is_err()` |
| `parse_unicode_escape_invalid_hex` | `\uZZZZ` | `is_err()` |
| `parse_unicode_escape_surrogate` | `\uD800` | `is_err()` |
| `parse_unterminated_multiline_string` | `"""unterminated` | `is_err()` |
| `from_json_invalid` | `{"a":1,}` (trailing comma) | `is_err()` |

### Success / Edge-Case Parse Tests (3 tests)

| Function | Input | Assertion |
|----------|-------|-----------|
| `parse_number_overflow_falls_to_json_number` | `18446744073709551616` | Parse succeeds; stored as `Value::JsonNumber` |
| `parse_deep_nesting_ok` | `[[[[[[[1]]]]]]]` | Parse succeeds; `get("root")` returns value |
| `from_json_root_array_is_preserved` | `[1,2,3]` | Stored under `"root"` key as `Value::Array` |

### Error Variant Coverage (5 tests)

Tests that exercise specific `Error` enum variants for code coverage:

| Function | What It Tests | Assertion |
|----------|--------------|-----------|
| `parse_unknown_struct_in_table` | `@table nonexistent` references undefined struct | `is_err()`; message contains struct name |
| `parse_unexpected_eof_unclosed_brace` | `obj: {x: 1,` | `is_err()`; message indicates EOF |
| `parse_unexpected_eof_unclosed_bracket` | `arr: [1, 2,` | `is_err()` |
| `reader_missing_field` | `reader.get("nonexistent")` on valid binary | `is_err()`; message contains key name |
| `from_json_large_number_falls_to_json_number` | `{"big": 18446744073709551616}` | Parsed as `Value::JsonNumber` |

### Type Coercion Tests (2 tests)

Validates spec §2.5 best-effort numeric coercion during binary compilation:

| Function | Input | Assertion |
|----------|-------|-----------|
| `writer_int_overflow_coerces_to_zero` | `int8` field with value `999` | Binary roundtrip produces `Value::Int(0)` |
| `writer_uint_negative_coerces_to_zero` | `uint8` field with value `-1` | Binary roundtrip produces `Value::UInt(0)` |

### Binary Reader Tests (4 tests)

| Function | Input | Assertion |
|----------|-------|-----------|
| `reader_rejects_bad_magic` | `[0x58, 0x58, 0x58, 0x58]` | `Reader::open().is_err()` |
| `reader_rejects_bad_version` | Valid magic + version 3 | `Reader::open().is_err()` |
| `load_invalid_file_errors` | `.tl` file with bad syntax | `TeaLeaf::load().is_err()` |
| `load_invalid_utf8_errors` | `[0xFF, 0xFE, 0xFA]` | `TeaLeaf::load().is_err()` |

### Binary Corruption Tests (12 tests)

Tests that take valid binary output, corrupt specific bytes, and verify the reader does not panic:

| Function | What It Corrupts |
|----------|-----------------|
| `reader_corrupted_magic_byte` | Flips first magic byte |
| `reader_corrupted_string_table_offset` | Points string table offset past EOF |
| `reader_truncated_string_table` | Truncates file right after header |
| `reader_oversized_string_count` | Sets string count to `u32::MAX` |
| `reader_oversized_section_count` | Sets section count to `u32::MAX` |
| `reader_corrupted_schema_count` | Sets schema count to `u32::MAX` |
| `reader_flipped_bytes_in_section_data` | Flips bytes in last 10 bytes of section data |
| `reader_truncated_compressed_data` | Removes last 20 bytes from compressed file |
| `reader_invalid_zlib_stream` | Overwrites data section with `0xBA` bytes |
| `reader_zero_length_file` | Empty `Vec<u8>` |
| `reader_just_magic_no_header` | Only `b"TLBX"` (4 bytes, no header) |
| `reader_corrupted_type_code` | Replaces a type code byte with `0xFE` |

All corruption tests assert no panic. Most also verify that `Reader::from_bytes()` or `reader.get()` either returns an error or handles the corruption gracefully.

### Compression Stress Tests (4 tests)

| Function | What It Tests |
|----------|--------------|
| `compression_at_threshold_boundary` | Data just over 64 bytes triggers compression attempt; roundtrip OK |
| `compression_skipped_when_not_beneficial` | High-entropy data: compressed file not much larger than raw |
| `compression_all_identical_bytes` | 10K zeros: compressed size < half of raw; roundtrip OK |
| `compression_below_threshold_stored_raw` | Small data with `compress=true`: stored raw (same size as uncompressed) |

### Soak / Large-Corpus Tests (8 tests)

Stress tests for parser, writer, and reader with large inputs:

| Function | Scale | What It Tests |
|----------|-------|--------------|
| `soak_deeply_nested_arrays` | 200 levels deep | Parser handles deep nesting without stack overflow |
| `soak_wide_object` | 10,000 fields | Parser and `Value::Object` handle wide objects |
| `soak_large_array` | 100,000 integers | Parser handles large arrays; first/last element correct |
| `soak_large_array_binary_roundtrip` | 100,000 integers | Compile + read roundtrip with compression |
| `soak_many_sections` | 5,000 top-level keys | Binary writer/reader handles many sections |
| `soak_many_schemas` | 500 `@struct` definitions | Schema table handles large schema counts |
| `soak_string_deduplication` | 15,000 strings (5K dupes) | String dedup in binary writer; roundtrip correct |
| `soak_long_string` | 1 MB string | Binary writer/reader handles large string values |

### Memory-Mapped Reader Tests (10 tests)

Validates `Reader::open_mmap()` produces identical results to `Reader::open()` and `Reader::from_bytes()`:

| Function | What It Tests |
|----------|--------------|
| `mmap_roundtrip_all_primitive_types` | Int, float, bool, string, timestamp via mmap |
| `mmap_roundtrip_containers` | Arrays, objects, nested arrays via mmap |
| `mmap_roundtrip_schemas` | `@struct` + `@table` data via mmap |
| `mmap_roundtrip_compressed` | 500-element compressed array via mmap |
| `mmap_vs_open_equivalence` | All keys: `open_mmap` values == `open` values |
| `mmap_vs_from_bytes_equivalence` | All keys: `open_mmap` values == `from_bytes` values |
| `mmap_large_file` | 50,000-element array via mmap |
| `mmap_nonexistent_file` | `open_mmap` on missing path returns error |
| `mmap_multiple_sections` | 100 sections via mmap; boundary keys correct |
| `mmap_string_dedup` | 100 identical string values via mmap; dedup preserved |

## Directory Structure

```
adversarial-tests/
├── inputs/
│   ├── tl/              # 13 crafted .tl files (11 error + 2 success)
│   ├── json/            # 6 crafted .json files
│   └── tlbx/            # 4 .tlbx files (used by CLI scripts, not Rust tests)
├── core-harness/
│   ├── tests/
│   │   └── adversarial.rs   # 58 Rust integration tests
│   └── Cargo.toml
├── dotnet-harness/          # C# harness using TeaLeaf bindings
├── scripts/
│   ├── run_core_harness.ps1
│   ├── run_cli_adversarial.ps1
│   └── run_dotnet_harness.ps1
├── results/                 # CLI test logs and outputs
└── README.md
```

## Adding New Tests

### 1. Add an Inline Test (preferred)

Most adversarial tests generate their inputs inline. This avoids stale fixture files and keeps the test self-contained:

```rust
#[test]
fn parse_new_error_case() {
    assert_parse_err("malformed: input here");
}
```

The `assert_parse_err` helper asserts that `TeaLeaf::parse(input).is_err()`.

### 2. For Binary Tests

Use the `make_valid_binary` helper to produce valid bytes, then corrupt them:

```rust
#[test]
fn reader_new_corruption_case() {
    let mut data = make_valid_binary("val: 42", false);
    data[0] ^= 0xFF; // corrupt something
    let result = Reader::from_bytes(data);
    // Assert no panic; error or graceful handling OK
    if let Ok(r) = result {
        let _ = r.get("val");
    }
}
```

### 3. Input File Tests (for CLI scripts)

Place malformed input in the appropriate subdirectory for CLI adversarial testing:

```
adversarial-tests/inputs/tl/bad_new_case.tl
```

The CLI script `run_cli_adversarial.ps1` exercises these files through the `tealeaf` CLI binary and logs results to `results/cli/`.
