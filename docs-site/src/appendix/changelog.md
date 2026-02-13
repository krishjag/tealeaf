# Changelog

## v2.0.0-beta.12 (Current)

### Features
- **Clap CLI migration** — Replaced hand-rolled `env::args()` parsing with `clap` v4 derive macros. Provides auto-generated colored help, typo suggestions (e.g., `compil` → "did you mean 'compile'?"), `--help`/`-h` on every subcommand, and `--version`/`-V` flag — all derived from annotated structs with zero manual usage text.
  - Added `clap = { version = "4", features = ["derive", "color"] }` and `clap_complete = "4"` dependencies
  - `try_parse()` pattern preserves exit code 1 for all errors (including no-args help via `arg_required_else_help`)
  - Deleted `print_usage()` function — clap auto-generates help from doc comments
- **Shell completions subcommand** — `tealeaf completions <shell>` generates tab-completion scripts for bash, zsh, fish, powershell, and elvish. Completes subcommands, flags, and `help` arguments.
- **`get_path` dot-path navigation** — Navigate nested values using dot-path expressions with array indexing.
  - Rust: `TeaLeaf::get_path("order.items[0].name")` at document level, `Value::get_path("items[0].price")` within values
  - FFI: `tl_document_get_path(doc, path)` and `tl_value_get_path(value, path)`
  - .NET: `TLDocument.GetPath(path)` and `TLValue.GetPath(path)` with null-safe returns
  - Path syntax: `key.field[N].field` — dot-separated field access, `[N]` for array indexing
- **Object field iteration API** — Enumerate object fields by index without knowing keys upfront.
  - FFI: `tl_value_object_len`, `tl_value_object_get_key_at`, `tl_value_object_get_value_at`
  - .NET: `TLValue.ObjectFieldCount`, `GetObjectKeyAt(index)`, `GetObjectValueAt(index)`, `AsObject()` for `(key, value)` tuples, `Keys` property, `GetEnumerator()` for `foreach` support on objects and arrays
- **Document schema introspection API** — Inspect schema definitions at runtime without parsing text.
  - FFI: `tl_document_schema_count`, `tl_document_schema_name`, `tl_document_schema_field_count`, `tl_document_schema_field_name`, `tl_document_schema_field_type`, `tl_document_schema_field_nullable`, `tl_document_schema_field_is_array` (7 new C functions)
  - .NET: `TLDocument.Schemas` (IReadOnlyList), `TLDocument.GetSchema(name)`, `TLDocument.SchemaCount`; `TLSchema` and `TLField` classes with `Name`, `Type`, `IsNullable`, `IsArray` properties
- **`TLDocumentBuilder`** (.NET) — Fluent API for building multi-key documents with schema support.
  - Scalar overloads: `Add(key, string|int|long|double|float|bool|DateTimeOffset)`
  - List overloads: `AddList(key, IEnumerable<string|int|double>)`
  - Object support: `Add<T>(key, value)` and `AddList<T>(key, items)` for `[TeaLeaf]`-attributed types
  - Document merging: `AddDocument(TLDocument)` with automatic schema deduplication
  - Builder pattern: all methods return `this` for chaining
  - `Build()` produces a `TLDocument` with merged schemas

### Bug Fixes
- **Schema deduplication in .NET source generator** — Diamond dependency scenarios (type A and type B both reference type C) no longer emit duplicate `@struct` definitions. New `CollectTeaLeafSchemas(StringBuilder, HashSet<string>)` method performs dependency-order traversal with a shared `emitted` set. Cross-assembly fallback detects types from referenced assemblies compiled with older generator versions.
- Fixed memory leak in `TLValue.AsMap()` where failed key/value pairs weren't disposed

### CLI
- Improved `--compact` description: "Omit insignificant whitespace for token-efficient output" (was "Use compact single-line output")
- Improved `--compact-floats` description: "Write whole-number floats as integers (e.g. 42.0 becomes 42)" (was "Use compact float representation")

### Testing
- Added 26 `TLDocumentBuilder` tests — single/multi-key documents, nested types, list support, `AddDocument` merging, JSON and binary round-trips, scalar overloads, method chaining
- Added 14 `GetPath` .NET tests — document and value-level navigation across 6-level nested objects with arrays and mixed types
- Added 15 adversarial `get_path` edge-case tests — empty string, nested fields, array indexing (valid/out-of-bounds/negative), unclosed brackets, empty brackets, non-numeric indices, double dots, leading/trailing dots, huge index overflow, scalar rejection
- Added `fuzz_get_path` fuzzer target — dedicated fuzzer splitting input between document and path strings, testing both `TeaLeaf::get_path` and `Value::get_path`
- Updated `fuzz_structured` fuzzer — added `get_path` coverage with arbitrary and well-formed paths
- Added 2 source generator deduplication tests — `SharedNestedType_DeduplicatesSchemas` (diamond) and `DeepDiamondDependency_DeduplicatesSchemas`
- Added .NET tests for `GetEnumerator()` on objects/arrays/scalars and `Keys` property
- Updated 3 CLI integration test assertions for clap error message format (`"Unknown command"` → `"frobnicate"` substring, `"unrecognized subcommand"` negative check)

### Documentation
- New [completions](../cli/completions.md) CLI reference page — usage, arguments, install instructions for 5 shells, completion coverage details
- Updated [CLI overview](../cli/overview.md) — added `completions` to command table
- Updated [SUMMARY.md](../SUMMARY.md) — added completions to CLI Reference navigation
- Updated [README.md](../../README.md) — added `completions` to command list
- Updated [TEALEAF_SPEC.md](../../spec/TEALEAF_SPEC.md) — added `completions` under new "Utilities" subsection in Section 7

---

## v2.0.0-beta.11

### Features
- **Optional/nullable field support in schema inference** — `analyze_array()` and `analyze_nested_objects()` no longer require strict field uniformity across all objects. Fields not present in every object are automatically marked nullable (`?`), enabling schema inference for real-world datasets where records share most fields but differ in optional ones (e.g., DCAT metadata, API responses). Requires at least 1 field present in all objects.
  - Union-based field collection: computes union of all field names across objects in an array, tracks per-field presence counts, marks fields below 100% presence as nullable
  - New `object_matches_schema()` helper allows nullable fields to be absent when matching objects to schemas — used in `write_value_with_schemas()` verification and `InferredType::to_field_type()` schema lookup
  - `InferredType::Object::merge()` now keeps intersection of common fields instead of returning `Mixed` when field counts differ
  - Nested array analysis collects ALL items from ALL parent objects (not just first representative), discovering the full field union across nested structures
  - Structural `@table` fallback guarded with `hint_name.is_some() || declared_type.is_some()` to prevent matching inside tuple values
- **Absent-field preservation** — Nullable fields with null/absent values (`~` in text, null-bit in binary) are omitted from the reconstructed object instead of being inserted as explicit `null`. This preserves the semantic distinction between "field absent" and "field explicitly null", ensuring JSON → TL → JSON roundtrip does not add spurious `"field": null` entries for fields that were simply missing in the original.
  - Text parser: `parse_tuple_with_schema()` skips inserting null for nullable fields
  - Binary reader: both bitmap decode paths skip inserting null for nullable fields
- **Most-complete object field ordering** — Schema inference now uses the field ordering from the object with the most fields (the "most representative" object) as the canonical schema field order, instead of first-seen ordering across all objects. Fields present only in other objects are appended at the end. This preserves the original JSON field ordering for the majority of records during roundtrip.

### Bug Fixes
- Fixed JSON → TL → JSON roundtrip adding extra `"field": null` entries for nullable fields that were absent in the original data
- Fixed field ordering loss during schema inference — first (often smallest) object determined schema field order, which didn't match the majority of objects

### Canonical Fixtures
- Updated `schemas.json`, `mixed_schemas.json`, `quoted_keys.json` — nullable fields with `~` in source now produce absent fields (no `"field": null`) in expected JSON output
- Recompiled `mixed_schemas.tlbx` to match updated binary reader behavior

### Testing
- Added `test_schema_inference_optional_fields` — array with optional field `c` marked as `int?`
- Added `test_schema_inference_optional_fields_roundtrip` — JSON → TL → JSON with absent optional fields preserved
- Added `test_schema_inference_no_common_fields_skipped` — `[{x:1}, {y:2}]` produces no schema (zero common fields)
- Added `test_schema_inference_single_common_field` — `[{id:1,a:2}, {id:3,b:4}]` infers schema with `id` common
- Added `test_schema_inference_optional_nested_array` — nested array field marked nullable when absent from some objects
- Added `test_schema_inference_optional_nested_object` — nested object field marked nullable when absent
- Added `test_schema_inference_wa_health_data_pattern` — pattern matching the WA health DCAT dataset structure
- Added `test_write_schemas_nullable_field_matching` — `@table` applied correctly when objects are missing nullable fields
- Added `test_schema_field_ordering_uses_most_complete_object` — most-fields object determines schema field order
- Added `test_schema_field_ordering_appends_extra_fields` — fields from smaller objects appended after representative
- Added `test_schema_field_ordering_roundtrip_preserves_order` — roundtrip preserves field ordering from most-complete object
- Updated `test_struct_with_nullable_field` — expects absent field instead of explicit null
- Updated .NET `StructArray_Table_BinaryRoundTrip_PreservesData` — expects `null` (absent) instead of `TLType.Null` for nullable fields in binary roundtrip

---

## v2.0.0-beta.10

### Features
- **Quoted field names in `@struct` definitions** — Schema inference no longer skips arrays of objects when field names contain special characters (`@`, `$`, `#`, `:`, `/`, spaces, etc.). Previously, `needs_quoting(field_name)` in `analyze_array()` and `analyze_nested_objects()` rejected the entire array, falling back to verbose inline map notation. Now the guard only checks the **schema name** (which appears unquoted in `@struct name(...)` and `@table name [...]`); field names that need quoting are emitted with quotes in the definition (e.g., `@struct record("@type":string, name:string)`).
  - Affects JSON-LD (`@type`, `@id`, `@context`), JSON Schema/OpenAPI (`$ref`, `$id`), OData (`@odata.type`, `@odata.id`), XML-to-JSON (`#text`, `#comment`), XML namespaces (`xsi:type`, `dc:title`), RDF/JSON (URI keys), and spreadsheet exports (space-separated keys)
  - Parser (`parse_struct_def()`) now accepts `TokenKind::String` in addition to `TokenKind::Word` for field names
  - Serialization required no changes — `write_key()` already quoted field names via `needs_quoting()`
- **Token counting script** (`accuracy-benchmark/scripts/count_tokens.py`) — Python script that uses Anthropic's `messages.count_tokens()` API to measure exact token usage for benchmark prompts across TL, JSON, and TOON formats without incurring completion costs. Generates per-task comparison table with TL vs JSON, TOON vs JSON, and TL vs TOON percentage columns, plus data-only token estimates (subtracting shared instruction overhead).

### Canonical Fixtures
- **New: `quoted_keys`** (15th canonical sample) — 8 schemas covering all special-character key patterns:
  - `jsonld_record` — `"@type"`, `"@id"` (JSON-LD)
  - `schema_def` — `"$ref"`, `"$id"` (JSON Schema / OpenAPI)
  - `xml_node` — `"#text"`, `"#comment"` (XML-to-JSON)
  - `ns_element` — `"xsi:type"`, `"dc:title"` (XML namespaces)
  - `odata_entity` — `"@odata.type"`, `"@odata.id"` (OData)
  - `rdf_triple` — `"http://schema.org/name"`, `"http://schema.org/age"` (RDF/JSON URIs)
  - `contact` — `"First Name"`, `"Last Name"` (space-separated keys)
  - `catalog_item` — mixed quoted (`"@type"`, `"$id"`, `"sku:code"`) and unquoted (`name`) fields

### Testing
- Added `test_schema_inference_with_at_prefixed_keys` — verifies JSON-LD `@type` fields trigger schema inference with quoted field names in `@struct` output
- Added `test_schema_inference_quoted_field_roundtrip` — full JSON → TL → JSON roundtrip with `@type` keys
- Added `test_schema_inference_skips_when_schema_name_needs_quoting` — confirms `@items` array key (which would produce schema name `@item`) correctly skips inference
- Added `test_schema_inference_root_array_with_at_keys` — root-level `@root-array` with `@type` keys
- Added `test_schema_inference_dollar_prefixed_keys` — `$ref`, `$id` (JSON Schema / OpenAPI)
- Added `test_schema_inference_hash_prefixed_keys` — `#text`, `#comment` (XML-to-JSON)
- Added `test_schema_inference_colon_in_keys` — `xsi:type`, `dc:title` (XML namespaces)
- Added `test_schema_inference_odata_keys` — `@odata.type`, `@odata.id` (OData)
- Added `test_schema_inference_uri_keys` — `http://schema.org/name` (RDF/JSON full URIs)
- Added `test_schema_inference_space_in_keys` — `First Name`, `Last Name` (spreadsheet exports)
- Added `test_schema_inference_mixed_special_keys` — `@type` + `$id` + `sku:code` + unquoted `name` in one schema, verifying only special-character fields are quoted
- Added `test_parse_struct_with_quoted_fields` — parser accepts `@struct foo("@type":string, name:string)` and correctly deserializes `@table` rows
- Added 5 canonical tests: `quoted_keys` text-to-JSON, binary roundtrip, full roundtrip (text → binary → JSON), compact roundtrip, compact-is-not-larger

### Known Limitations (Resolved)
- ~~JSON import does not recognize `$ref`, `$tag`, or timestamp strings~~ — `$ref` and other special-character keys now get full schema inference and positional compression (timestamps and `$tag` remain unrecognized)

---

## v2.0.0-beta.9

### Features
- **`FormatOptions` struct with `compact_floats`** — new `FormatOptions` struct replaces bare `compact: bool` throughout the serialization pipeline, adding `compact_floats` to strip `.0` from whole-number floats (e.g., `35934000000.0` → `35934000000`). Saves additional characters/tokens for financial and scientific datasets. Trade-off: re-parsing produces `Int` instead of `Float` for whole-number values.
  - Rust: `FormatOptions::compact().with_compact_floats()` via `doc.to_tl_with_options(&opts)`
  - CLI: `--compact-floats` flag on `from-json` and `decompile` commands
  - FFI: `tl_document_to_text_with_options(doc, compact, compact_floats)` and `tl_document_to_text_data_only_with_options(doc, compact, compact_floats)`
  - .NET: unified `doc.ToText(compact, compactFloats, ignoreSchemas)` with all-optional parameters
- **Lexer/parser refactor for colon handling** — removed `Tag(String)` token kind from lexer; colon now always emits as `Colon` token. Tags (`:name value`) are parsed in the parser as `Colon + Word` sequence. This enables no-space syntax: `key:value` and `key::tag` now parse correctly without requiring a space after the colon.

### .NET
- Source generator: `ResolveStructName()` — nested types now honor `[TeaLeaf(StructName = "...")]` attribute when resolving struct names for arrays, lists, and nested objects. Previously always used `ToSnakeCase(type.Name)`, ignoring the override.
- Unified `ToText()` and `ToTextDataOnly()` into a single `ToText(bool compact = false, bool compactFloats = false, bool ignoreSchemas = false)` method with all-optional parameters. Replaces the previous 4-method API (`ToText`, `ToTextDataOnly`, `ToTextCompact`, `ToTextCompactDataOnly`) with named-parameter calling style (e.g., `doc.ToText(compact: true)`, `doc.ToText(ignoreSchemas: true)`).

### Accuracy Benchmark
- **Real dataset support** — benchmark now supports real-world datasets alongside synthetic data. Starting with finance domain using SEC EDGAR 2025 Q4 10-K filings (AAPL, MSFT, GOOGL, AMZN, NVDA).
- **Three-format comparison in `analysis.tl`** — `TLWriter` now emits `@struct` schema definitions for all table types (`api_response`, `analysis_result`, `comparison_result`) and two new format comparison tables (`format_accuracy`, `format_tokens`) when `--compare-formats` is used. String values in table rows are properly quoted for valid TeaLeaf parsing.
- Reorganized task data into `synthetic-data/` subdirectories to separate from real datasets.
- Added task configuration files (`real.json`, `synthetic.json`) for independent benchmark runs.
- Added utility examples: `convert_formats.rs`, `tl_roundtrip.rs`, `toon_roundtrip.rs`.
- `convert_json_to_tl()` now uses `FormatOptions::compact().with_compact_floats()` for maximum token savings.

### Testing
- Added `test_format_float_compact_floats` — verifies whole-number stripping, non-whole preservation, special value handling (NaN/inf), and scientific notation passthrough
- Added `test_dumps_with_compact_floats` — integration test for `FormatOptions` with mixed int/float data
- Added `test_colon_then_word` — verifies lexer emits `Colon + Word` instead of `Tag` for `:Circle` syntax
- Added `test_tagged_value_no_space_after_colon` — verifies `key::tag value` parsing
- Added `test_key_value_no_space_after_colon` — verifies `key:value` parsing without spaces
- Added 4 source generator tests for `ResolveStructName`: nested type with `StructName` override, list/array of nested type with override, and default snake_case fallback
- Added 4 .NET tests for compact text and unified `ToText` API: `ToTextCompact_RemovesInsignificantWhitespace`, `ToTextCompact_WithSchemas_IsSmallerThanPretty`, `ToTextCompact_RoundTrips`, `ToText_IgnoreSchemas_ExcludesSchemas`
- Updated 4 fuzz targets (`fuzz_serialize`, `fuzz_parse`, `fuzz_structured`, `fuzz_json_schemas`) with compact and `compact_floats` roundtrip coverage and `values_numeric_equal` comparator for Float↔Int/UInt coercion

### Documentation
- Added [Compact Floats: Intentional Lossy Optimization](../guides/round-trip.md#compact-floats-intentional-lossy-optimization) section to round-trip fidelity guide
- Updated CLI docs for `--compact-floats` flag on `decompile` and `from-json`
- Updated Rust overview with `FormatOptions` section and `to_tl_with_options` API
- Updated FFI API reference with `tl_document_to_text_with_options` and `tl_document_to_text_data_only_with_options`
- Updated .NET overview with new `ToText`/`ToTextDataOnly` overloads
- Updated LLM context guide with `FormatOptions` examples and compaction options table
- Updated crates.io and NuGet README files
- Updated accuracy benchmark documentation with `analysis.tl` structure, three-format comparison tables, and latest benchmark results (~43% input token savings on real-world data)
- Updated token savings claims across README.md, tealeaf-core/README.md, introduction.md, and CLAUDE.md to reflect latest benchmark data

---

## v2.0.0-beta.8

### .NET
- **XML documentation in NuGet packages** — `TeaLeaf` and `TeaLeaf.Annotations` packages now include XML doc files (`TeaLeaf.xml`, `TeaLeaf.Annotations.xml`) for all target frameworks. Consumers get IntelliSense tooltips for all public APIs. Previously, `GenerateDocumentationFile` was not enabled and the `.xml` files were absent from the `.nupkg`.
- Added XML doc comments to all undocumented public members: `TLType` enum values (13), `TLDocument.ToString`/`Dispose`, `TLReader.Dispose`, `TLField.ToString`, `TLSchema.ToString`, `TLException` constructors (3)
- Enabled `TreatWarningsAsErrors` for `TeaLeaf` and `TeaLeaf.Annotations` — missing XML docs or other warnings are now compile errors, preventing regressions

### Testing
- Added `ToJson_PreservesSpecialCharacters_NoUnicodeEscaping` — verifies `+`, `<`, `>`, `'` survive binary round-trip without Unicode escaping in both `ToJson()` and `ToJsonCompact()` paths
- Added `ToJson_PreservesFloatDecimalPoint_WholeNumbers` — verifies whole-number floats (`99.0`, `150.0`, `0.0`) retain `.0` suffix and non-whole floats (`4.5`, `3.75`) preserve decimal digits

---

## v2.0.0-beta.7

### .NET
- Fixed `TLReader.ToJson()` escaping non-ASCII-safe characters — `+` in phone numbers rendered as `\u002B`, `<`/`>` as `\u003C`/`\u003E`, etc. `System.Text.Json`'s default `JavaScriptEncoder.Default` HTML-encodes these characters for XSS safety, which is inappropriate for a data serialization library. All three JSON serialization methods (`ToJson`, `ToJsonCompact`, `GetAsJson`) now use `JavaScriptEncoder.UnsafeRelaxedJsonEscaping` via shared `static readonly` options.
- Fixed `TLReader.ToJson()` dropping `.0` suffix from whole-number floats — `3582.0` in source JSON became `3582` after binary round-trip because `System.Text.Json`'s `JsonValue.Create(double)` strips trailing `.0`. Added `FloatToJsonNode` helper that uses `F1` formatting for whole-number doubles, preserving formatting fidelity with the Rust CLI path.

---

## v2.0.0-beta.6

### Features
- **Recursive array schema inference in JSON import** — `from_json_with_schemas` now discovers schemas for arrays nested inside objects at arbitrary depth (e.g., `items[].product.stock[]`). Previously, `analyze_nested_objects` only recursed into nested objects but not nested arrays, causing deeply nested arrays to fall back to `[]any`. The CLI and derive-macro paths now produce equivalent schema coverage.
- **Deterministic schema declaration order** — `analyze_array` and `analyze_nested_objects` now use single-pass field-order traversal (depth-first), matching the derive macro's field-declaration-order strategy. Previously, both functions made two separate passes (arrays first, then objects), causing schema declarations to appear in a different order than the derive/Builder API path. CLI and Builder API now produce byte-identical `.tl` output for the same data.

### Bug Fixes
- Fixed binary encoding corruption for `[]any` typed arrays — `encode_typed_value` incorrectly wrote `TLType::Struct` as the element type for the "any" pseudo-type (the `to_tl_type()` default for unknown names), causing the reader to interpret heterogeneous data as struct schema indices. Arrays with mixed element types inside schema-typed objects (e.g., `order.customer`, `order.payment`) now correctly use heterogeneous `0xFF` encoding when no matching schema exists.

### Tooling
- Version sync scripts (`sync-version.ps1`, `sync-version.sh`) now regenerate the workflow diagram (`assets/tealeaf_workflow.png`) via `generate_workflow_diagram.py` on each version bump

### Testing
- Added `json_any_array_binary_roundtrip` — focused regression test verifying `[]any` fields inside schema-typed structs survive binary compilation with full data integrity verification
- Added `retail_orders_json_binary_roundtrip` — end-to-end test exercising JSON → infer schemas → compile → binary read with `retail_orders.json` (the exact path that was untested)
- Added .NET `FromJson_HeterogeneousArrayInStruct_BinaryRoundTrips` — mirrors the Rust `[]any` regression test through the FFI layer
- Strengthened .NET `FromJson_RetailOrdersFixture_CompileRoundTrips` — upgraded from string-contains check to structural JSON verification (10 orders, 4 products, 3 customers, spot-check order ID and item count)
- Added `json_inference_nested_array_inside_object` — verifies arrays nested inside objects (e.g., `items[].product.stock[]`) get their own schema and typed array fields
- Added `gen_retail_orders_api_tl` derive integration test — generates `.tl` from Rust DTOs via Builder API and confirms byte-identical output with CLI path
- Added `examples/retail_orders_different_shape_cli.tl` and `retail_orders_different_shape_api.tl` comparison fixtures (2,395 bytes each, zero diff)
- Moved `retail_orders_different_shape.rs` from `examples/` to `tealeaf-core/tests/fixtures/` to keep test dependencies within the crate boundary
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

