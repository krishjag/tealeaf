# Adversarial Tests

The adversarial test suite validates TeaLeaf's error handling and robustness using crafted malformed inputs and edge cases. All tests are isolated in the `adversarial-tests/` directory to avoid touching core project files.

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
```

## Test Input Categories

### TeaLeaf Format (.tl)

Crafted `.tl` files testing parser error paths:

| File | Error Tested | Expected |
|------|-------------|----------|
| `bad_unclosed_string.tl` | Unclosed string literal (`"Alice`) | Parse error |
| `bad_missing_colon.tl` | Missing colon in key-value pair | Parse error |
| `bad_invalid_escape.tl` | Invalid escape sequence (`\q`) | Parse error |
| `bad_number_overflow.tl` | Number exceeding u64 bounds | Parse error |
| `bad_table_wrong_arity.tl` | Table row with wrong field count | Parse error |
| `bad_schema_unclosed.tl` | Unclosed `@struct` definition | Parse error |
| `bad_unicode_escape_short.tl` | Incomplete `\u` escape (`\u12`) | Parse error |
| `bad_unicode_escape_invalid_hex.tl` | Invalid hex in `\uZZZZ` | Parse error |
| `bad_unicode_escape_surrogate.tl` | Unicode surrogate pair (`\uD800`) | Parse error |
| `bad_unterminated_multiline.tl` | Unterminated `"""` multiline string | Parse error |
| `invalid_utf8.tl` | Invalid UTF-8 byte sequence | Parse error |

Edge cases that should succeed:

| File | What It Tests | Expected |
|------|--------------|----------|
| `deep_nesting.tl` | 7 levels of nested arrays (`[[[[[[[1]]]]]]]`) | Parse OK |
| `empty_doc.tl` | Empty document | Parse OK |

### JSON Format (.json)

Files testing `from_json` error and edge-case paths:

| File | What It Tests | Expected |
|------|--------------|----------|
| `invalid_json_trailing.json` | Trailing comma or content | Parse error |
| `invalid_json_unclosed.json` | Unclosed object or array | Parse error |
| `large_number.json` | Number overflowing f64 | Falls to float |
| `deep_array.json` | Deeply nested arrays | Parse OK |
| `empty_object.json` | Empty JSON object `{}` | Parse OK |
| `root_array.json` | Root-level array `[1,2,3]` | Preserved as array |

### Binary Format (.tlbx)

Malformed binary files testing the reader:

| File | What It Tests | Expected |
|------|--------------|----------|
| `bad_magic.tlbx` | Invalid magic bytes (not `TLBX`) | Reader error |
| `bad_version.tlbx` | Invalid version field (e.g., version 3) | Reader error |
| `random_garbage.tlbx` | Random bytes | Reader error |
| `truncated_header.tlbx` | Incomplete 64-byte header | Reader error |

## Test Functions

### Parse Error Tests

| Function | Input | Assertion |
|----------|-------|-----------|
| `parse_invalid_syntax_unclosed_string` | `name: "Alice` | `TeaLeaf::parse().is_err()` |
| `parse_invalid_escape_sequence` | `name: "Alice\q"` | `TeaLeaf::parse().is_err()` |
| `parse_missing_colon` | `name "Alice"` | `TeaLeaf::parse().is_err()` |
| `parse_schema_unclosed` | Unclosed `@struct` | `TeaLeaf::parse().is_err()` |
| `parse_table_wrong_arity` | 3 fields for 2-field schema | `TeaLeaf::parse().is_err()` |
| `parse_number_overflow` | `18446744073709551616` | `TeaLeaf::parse().is_err()` |
| `parse_unicode_escape_short` | `\u12` | `TeaLeaf::parse().is_err()` |
| `parse_unicode_escape_invalid_hex` | `\uZZZZ` | `TeaLeaf::parse().is_err()` |
| `parse_unicode_escape_surrogate` | `\uD800` | `TeaLeaf::parse().is_err()` |
| `parse_unterminated_multiline_string` | `"""unterminated` | `TeaLeaf::parse().is_err()` |

### Success Case Tests

| Function | Input | Assertion |
|----------|-------|-----------|
| `parse_deep_nesting_ok` | `[[[[[[[1]]]]]]]` | Parse succeeds, `get("root")` returns value |
| `from_json_large_number_falls_to_float` | `18446744073709551616` | Parsed as `Value::Float` |
| `from_json_root_array_is_preserved` | `[1,2,3]` | Stored under `"root"` key as `Value::Array` |

### Binary Format Tests

| Function | Input | Assertion |
|----------|-------|-----------|
| `reader_rejects_bad_magic` | `[0x58, 0x58, 0x58, 0x58]` | `Reader::open().is_err()` |
| `reader_rejects_bad_version` | Valid magic + version 3 | `Reader::open().is_err()` |
| `load_invalid_file_errors` | `.tl` file with bad syntax | `TeaLeaf::load().is_err()` |
| `load_invalid_utf8_errors` | `[0xFF, 0xFE, 0xFA]` | `TeaLeaf::load().is_err()` |

## Directory Structure

```
adversarial-tests/
├── inputs/
│   ├── tl/              # 11 crafted .tl files
│   ├── json/            # 6 crafted .json files
│   └── tlbx/            # 4 crafted .tlbx files
├── core-harness/
│   ├── tests/
│   │   └── adversarial.rs   # Rust integration tests
│   └── Cargo.toml
├── dotnet-harness/          # C# harness using TeaLeaf bindings
├── scripts/
│   ├── run_core_harness.ps1
│   └── run_cli_adversarial.ps1
├── results/                 # Test logs and outputs
└── README.md
```

## Adding New Tests

### 1. Create an Input File

Place malformed input in the appropriate subdirectory:

```
adversarial-tests/inputs/tl/bad_new_case.tl
```

### 2. Add a Test Function

In `adversarial-tests/core-harness/tests/adversarial.rs`:

```rust
#[test]
fn parse_new_error_case() {
    let input = std::fs::read_to_string("../inputs/tl/bad_new_case.tl").unwrap();
    assert!(TeaLeaf::parse(&input).is_err());
}
```

Or for inline tests:

```rust
#[test]
fn parse_new_inline_case() {
    assert_parse_err("malformed: input here");
}
```

The `assert_parse_err` helper asserts that `TeaLeaf::parse(input).is_err()`.
