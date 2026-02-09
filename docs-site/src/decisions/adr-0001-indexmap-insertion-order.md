# ADR-0001: Use IndexMap for Insertion Order Preservation

- **Status:** Accepted
- **Date:** 2026-02-05
- **Applies to:** tealeaf-core, tealeaf-derive, tealeaf-ffi

## Context

TeaLeaf's primary use case is context engineering for LLM applications, where structured data passes through multiple format conversions (JSON → `.tl` → `.tlbx` and back). Users intentionally order their JSON keys to convey semantic meaning — for example, placing `name` before `description` before `details` to mirror how a human would read the document. Prior to this change, all user-facing maps used `HashMap<K, V>`, and the text serializer and binary writer explicitly sorted keys alphabetically before output.

This caused two problems:

1. **Semantic ordering was lost.** A user who wrote `{"zebra": 1, "apple": 2}` in their JSON would get `{"apple": 2, "zebra": 1}` after a round-trip through TeaLeaf. For LLM prompt engineering, this reordering could change how models interpret the context.

2. **Sorting was unnecessary work.** Every serialization path (`dumps()`, `compile()`, `write_value()`, `to_tl_with_schemas()`) collected keys into a `Vec`, sorted them, and then iterated — adding O(n log n) overhead to every output operation.

### Alternatives Considered

| Approach | Pros | Cons |
|----------|------|------|
| **Keep HashMap + sort** (status quo) | Deterministic output, no dependency change | Loses user intent, sorting overhead |
| **Vec of (key, value) pairs** | Order preserved, no new dependency | Loses O(1) key lookup, breaks API surface broadly |
| **IndexMap** | Order preserved, O(1) lookup, drop-in API | Slightly slower decode (insertion cost), new dependency |
| **BTreeMap** | Sorted + deterministic | Still not insertion-ordered, lookup O(log n) |

## Decision

Replace `HashMap` with `IndexMap` (from the `indexmap` crate v2) in all **user-facing ordered containers**:

- `Value::Object` → `ObjectMap<String, Value>` (type alias for `IndexMap`)
- `TeaLeaf.data`, `TeaLeaf.schemas`, `TeaLeaf.unions` → `IndexMap<String, _>`
- `Parser` output, `Reader.sections`, trait return types → `IndexMap`

**Internal lookup tables stay as `HashMap`** because they don't need ordering:

- `Writer.string_map`, `Writer.schema_map`, `Writer.union_map`
- `Reader.schema_map`, `Reader.union_map`, `Reader.cache`

Additionally:

- Enable `serde_json`'s `preserve_order` feature so JSON parsing also preserves key order
- Remove all explicit `keys.sort()` calls from serialization paths
- Re-export `IndexMap` and `ObjectMap` from `tealeaf-core` so derive macros and downstream crates don't need a direct `indexmap` dependency

## Consequences

### Positive

- **Round-trip fidelity.** JSON → TeaLeaf → JSON now preserves the original key order at every level (sections, object fields, schema definitions).
- **Encoding is faster.** Removing O(n log n) sort calls from every serialization path yields measurable improvements in encode benchmarks (6–17% for small/medium objects).
- **Simpler serialization code.** Serialization loops iterate the map directly instead of collecting-sorting-iterating.
- **Binary format is unchanged.** Old `.tlbx` files remain fully readable. The reader always produces keys in file order, which for old files happens to be alphabetical.

### Negative

- **Binary decode is slower.** `IndexMap::insert()` is slower than `HashMap::insert()` because it maintains a dense insertion-order array alongside the hash table. Benchmarks show **+56% to +105% regression** for decode-heavy workloads (large arrays of objects, deeply nested structs). For the primary use case (LLM context), this is acceptable because:
  - Documents are typically encoded once and consumed as text (not repeatedly decoded from binary)
  - The absolute times remain in the microsecond-to-millisecond range
  - Encode performance (the more common hot path) improved

- **New dependency.** `indexmap` v2 is a well-maintained, widely-used crate (used by `serde_json` internally), so supply-chain risk is minimal.

- **Public API change.** `TeaLeaf::new()` now takes `IndexMap` instead of `HashMap`. This is a breaking change, mitigated by:
  - The project is in beta (`2.0.0-beta.2`)
  - `From<HashMap<String, Value>> for Value` conversion is retained for backward compatibility
  - Downstream code using `.get()`, `.insert()`, `.iter()` works identically

### Benchmark Summary

| Workload | Encode | Decode |
|----------|--------|--------|
| small_object | -16% (faster) | — |
| nested_structs | -10% to -17% (faster) | +56% to +68% (slower) |
| large_array_10000 | -5% (faster) | +105% (slower) |
| tabular_5000 | -69% (faster) | -48% (faster) |

> **Note:** Tabular workloads use struct-array encoding (columnar), which has fewer per-row `IndexMap` insertions. The decode regression is concentrated in generic object decoding where each row creates a new `ObjectMap` with field-by-field inserts.

## References

- [`indexmap` crate documentation](https://docs.rs/indexmap/latest/indexmap/)
- [serde_json `preserve_order` feature](https://docs.rs/serde_json/latest/serde_json/#feature-flags)
- Implementation PR: HashMap → IndexMap migration across 16+ files
