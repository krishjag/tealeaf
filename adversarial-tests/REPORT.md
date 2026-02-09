# Adversarial Test Report

## Executive Summary
All adversarial suites passed after extending the corpus and updating harness expectations to match JSON parity and escape handling:
- Rust core harness: PASS (18 tests)
- CLI adversarial suite: PASS
- .NET harness: PASS

## Environment
- Cargo: 1.92.0 (344c4567c 2025-10-21)
- .NET SDK: 10.0.102
- Build configuration: Release (native + .NET)

## Suites and Results

### 1) Rust Core Harness
Command:
- powershell -ExecutionPolicy Bypass -File adversarial-tests\scripts\run_core_harness.ps1

Tests executed (18):
- parse_invalid_syntax_unclosed_string (expect error)
- parse_invalid_escape_sequence (expect error)
- parse_missing_colon (expect error)
- parse_schema_unclosed (expect error)
- parse_table_wrong_arity (expect error)
- parse_number_overflow (expect error)
- parse_unicode_escape_short (expect error)
- parse_unicode_escape_invalid_hex (expect error)
- parse_unicode_escape_surrogate (expect error)
- parse_unterminated_multiline_string (expect error)
- parse_deep_nesting_ok (expect success)
- from_json_invalid (expect error)
- from_json_large_number_falls_to_float (expect success, Value::Float)
- from_json_root_array_is_preserved (expect success, root array)
- reader_rejects_bad_magic (expect error)
- reader_rejects_bad_version (expect error)
- load_invalid_file_errors (expect error)
- load_invalid_utf8_errors (expect error)

Result: PASS

### 2) CLI Adversarial Suite
Command:
- powershell -ExecutionPolicy Bypass -File adversarial-tests\scripts\run_cli_adversarial.ps1

Valid baseline cases (expect success):
- validate retail_orders.tl
- compile retail_orders.tl -> retail_orders.tlbx
- decompile retail_orders.tlbx -> retail_orders_decompiled.tl
- to-json retail_orders.tl -> retail_orders.json
- json-to-tlbx retail_orders.json -> retail_orders_from_json.tlbx

Adversarial TL cases (expect error):
- bad_unclosed_string.tl
- bad_missing_colon.tl
- bad_invalid_escape.tl
- bad_number_overflow.tl
- bad_table_wrong_arity.tl
- bad_schema_unclosed.tl
- bad_unicode_escape_short.tl
- bad_unicode_escape_invalid_hex.tl
- bad_unicode_escape_surrogate.tl
- bad_unterminated_multiline.tl
- invalid_utf8.tl

Other TL case (expect success):
- deep_nesting.tl

Adversarial JSON cases:
- invalid_json_trailing.json (expect error)
- invalid_json_unclosed.json (expect error)
- large_number.json (expect success; coerces to float per JSON parity)
- deep_array.json (expect success)
- root_array.json (expect success)
- empty_object.json (expect success)

Adversarial TLBX cases (expect error):
- bad_magic.tlbx
- truncated_header.tlbx
- bad_version.tlbx
- random_garbage.tlbx (tlbx-to-json)

Result: PASS
Logs:
- adversarial-tests\results\cli\*.log

Execution excerpt:
- CLI adversarial tests passed.

### 3) .NET Adversarial Harness
Command:
- powershell -ExecutionPolicy Bypass -File adversarial-tests\scripts\run_dotnet_harness.ps1

Cases (expectation):
- parse_invalid_unclosed_string (error)
- parse_invalid_escape (error)
- parse_unicode_escape_short (error)
- parse_unicode_escape_invalid_hex (error)
- parse_unicode_escape_surrogate (error)
- parse_unterminated_multiline (error)
- parse_missing_colon (error)
- parse_file_invalid (error)
- parse_file_invalid_utf8 (error)
- from_json_invalid (error)
- from_json_large_number_overflow (success, TLType.Float)
- from_json_root_array (success; root key present)
- parse_deep_nesting_ok (success)

Result: PASS

Execution excerpt:
- Adversarial harness passed.

## Execution Output Summary (captured)
- Core harness: "running 18 tests" ... "test result: ok"
- CLI suite: "CLI adversarial tests passed."
- .NET harness: "Adversarial harness passed."

## Input Corpus Inventory

### TL Inputs
- bad_unclosed_string.tl
- bad_missing_colon.tl
- bad_invalid_escape.tl
- bad_number_overflow.tl
- bad_table_wrong_arity.tl
- bad_schema_unclosed.tl
- deep_nesting.tl
- empty_doc.tl
- bad_unicode_escape_short.tl
- bad_unicode_escape_invalid_hex.tl
- bad_unicode_escape_surrogate.tl
- bad_unterminated_multiline.tl
- invalid_utf8.tl

### JSON Inputs
- invalid_json_trailing.json
- invalid_json_unclosed.json
- large_number.json
- deep_array.json
- root_array.json
- empty_object.json

### TLBX Inputs
- bad_magic.tlbx
- truncated_header.tlbx
- random_garbage.tlbx
- bad_version.tlbx

## Artifacts and Locations
- Core harness: adversarial-tests\core-harness\tests\adversarial.rs
- .NET harness: adversarial-tests\dotnet-harness\Program.cs
- CLI runner: adversarial-tests\scripts\run_cli_adversarial.ps1
- Core runner: adversarial-tests\scripts\run_core_harness.ps1
- .NET runner: adversarial-tests\scripts\run_dotnet_harness.ps1
- CLI logs: adversarial-tests\results\cli\*.log

## Notes
- JSON parity fixes confirmed: invalid escape sequences now error; numbers follow i64 -> u64 -> f64 cascade.
- .NET harness runner rebuilds native libraries and copies the freshest native DLL into both net8.0 and net10.0 output directories.
- Cross-platform RID detection is supported in the .NET harness runner.

## Gaps / Future Extensions
- Large file fuzzing and performance thresholds (e.g., deep nesting beyond current limits).
- More binary corruption patterns (string table overflow, invalid schema indices, oversized offsets).
- Mixed newline handling and BOM-prefixed inputs.
- Windows vs. Unix path edge cases in CLI.
