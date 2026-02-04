# Specification Governance

How the TeaLeaf specification, implementation, and tests relate to each other.

## Two Sources of Truth

TeaLeaf has a prose specification and an executable specification:

| | Prose Spec | Executable Spec |
|---|---|---|
| **Location** | `spec/TEALEAF_SPEC.md` | `canonical/` test suite |
| **Format** | Markdown document | `.tl` samples + expected JSON + pre-compiled `.tlbx` |
| **Enforced by** | Human review | CI (automated on every push and PR) |
| **Covers** | Full grammar, type system, binary layout | 14 feature areas, 52 tests (42 success + 10 error) |

The canonical test suite is the **normative** specification. If the prose spec and the tests disagree, the tests are authoritative. The prose spec is documentation that describes intent and rationale.

## What the Canonical Suite Validates

Each canonical sample is tested through three paths:

```
Text (.tl) ──────────────────────────────► JSON    (compare with expected/)
Binary (.tlbx) ──────────────────────────► JSON    (compare with expected/)
Text (.tl) ──► Binary (.tlbx) ──► Read ──► JSON    (full round-trip)
```

The 14 sample files cover:

| File | Coverage |
|------|----------|
| `primitives.tl` | Null, bool, int, float, string, escape sequences |
| `arrays.tl` | Empty, typed, mixed, nested arrays |
| `objects.tl` | Empty, simple, nested, deeply nested objects |
| `schemas.tl` | `@struct`, `@table`, nested structs, nullable fields |
| `special_types.tl` | References, tagged values, maps, edge cases |
| `timestamps.tl` | ISO 8601 variants, timezones, milliseconds |
| `numbers_extended.tl` | Hex, binary, scientific notation, i64 limits |
| `unions.tl` | `@union`, empty/multi-field variants |
| `multiline_strings.tl` | Triple-quoted strings, auto-dedent, code blocks |
| `unicode_escaping.tl` | CJK, Cyrillic, Arabic, emoji, ZWJ sequences |
| `refs_tags_maps.tl` | References, tagged values, maps, compositions |
| `mixed_schemas.tl` | Schema-bound and schemaless data together |
| `large_data.tl` | Stress tests: 100+ element arrays, deep nesting, long strings |
| `cyclic_refs.tl` | Reference cycles, forward references, self-references |

Error tests in `canonical/errors/` validate that invalid input produces specific, stable error messages across all interfaces (CLI, FFI, .NET).

## Change Process

### Adding New Behavior

When adding new syntax, types, or features:

1. **Design** -- Describe the change in an issue or PR description
2. **Implement** -- Modify the parser/encoder/decoder in `tealeaf-core`
3. **Add canonical tests** -- Create or extend a sample in `canonical/samples/`, generate expected JSON and binary fixtures
4. **Update the prose spec** -- Update `spec/TEALEAF_SPEC.md` to document the new behavior
5. **CI validates** -- All three round-trip paths must pass

A PR that adds implementation without canonical tests is incomplete. A PR that updates the prose spec without tests is documentation-only and does not change behavior.

### Modifying Existing Behavior

Behavior changes fall into two categories:

**Non-breaking** (output changes, error message improvements):
- Update canonical expected outputs (`canonical/expected/*.json`)
- Update error golden tests if error messages changed
- Update the prose spec

**Breaking** (syntax changes, binary format changes, type system changes):
- Requires a version bump in `release.json`
- Regenerate all binary fixtures (`canonical/binary/*.tlbx`)
- Update the prose spec with a clear note about the breaking change
- Binary format changes must update the format version constant in `writer.rs`

### Error Message Stability

Error messages are part of the public contract. The `canonical/errors/` directory contains invalid input files paired with expected error messages in `expected_errors.json`. Changes to error text should be noted in the changelog and may require downstream consumers to update.

## What Is Not Covered

The canonical suite focuses on the core format. These areas rely on their own test suites:

| Area | Test Location | Notes |
|------|--------------|-------|
| CLI flags and output formatting | `tealeaf-core/tests/cli_integration.rs` | Tests CLI behavior, not format correctness |
| Derive macros (Rust) | `tealeaf-core/tests/derive.rs` | Tests DTO conversion, not parsing |
| FFI memory management | `tealeaf-ffi` unit tests | Tests allocation/deallocation, not format |
| .NET source generator | `TeaLeaf.Generators.Tests` | Tests code generation, not format |
| .NET serialization | `TeaLeaf.Tests` | Tests managed-to-native bridge |
| Accuracy benchmark | `accuracy-benchmark` | Tests LLM accuracy, not format |

## Spec Versioning

The format version is embedded in the binary header (see `writer.rs`). The prose spec documents the current version. When the binary format changes in a backward-incompatible way:

1. The format version constant in `writer.rs` must be incremented
2. The reader (`reader.rs`) should handle both old and new versions where feasible
3. All binary fixtures in `canonical/binary/` must be regenerated
4. The prose spec must document the version change

The project version (`release.json`) and the binary format version are independent. A project version bump does not necessarily mean a format version bump, and vice versa.
