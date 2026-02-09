# Benchmarks

TeaLeaf includes a Criterion-based benchmark suite that measures encode/decode performance and output size across multiple serialization formats.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench -p tealeaf-core

# Run a specific scenario
cargo bench -p tealeaf-core -- small_object
cargo bench -p tealeaf-core -- large_array_1000
cargo bench -p tealeaf-core -- tabular_5000

# List available benchmarks
cargo bench -p tealeaf-core -- --list
```

Results are saved to `target/criterion/` with HTML reports and JSON data. Criterion tracks historical performance across runs.

## Formats Compared

Each scenario benchmarks encode and decode across six formats:

| Format | Library | Notes |
|--------|---------|-------|
| **TeaLeaf Parse** | `tealeaf` | Text parsing (`.tl` → in-memory) |
| **TeaLeaf Binary** | `tealeaf` | Binary compile/read (`.tlbx`) |
| **JSON** | `serde_json` | Standard JSON serialization |
| **MessagePack** | `rmp_serde` | Binary, schemaless |
| **CBOR** | `ciborium` | Binary, schemaless |
| **Protobuf** | `prost` | Binary with generated code from `.proto` definitions |

> **Note:** Protobuf benchmarks use `prost` with code generation via `build.rs`. The generated structs have known field offsets at compile time, giving Protobuf a structural speed advantage over TeaLeaf's dynamic key-based access.

## Benchmark Scenarios

| Group | Data Shape | Sizes | What It Tests |
|-------|-----------|-------|---------------|
| `small_object` | Config-like object | 1 | Header overhead, small payload efficiency |
| `large_array_100` | Array of Point structs | 100 | Array encoding at small scale |
| `large_array_1000` | Array of Point structs | 1,000 | Array encoding at medium scale |
| `large_array_10000` | Array of Point structs | 10,000 | Array encoding at large scale, throughput |
| `nested_structs` | Nested objects | 2 levels | Nesting overhead |
| `nested_structs_100` | Nested objects | 100 levels | Deep nesting scalability |
| `mixed_types` | Heterogeneous data | 1 | Strings, numbers, booleans mixed |
| `tabular_100` | `@table` User records | 100 | Schema-bound tabular data, small |
| `tabular_1000` | `@table` User records | 1,000 | Schema-bound tabular data, medium |
| `tabular_5000` | `@table` User records | 5,000 | Schema-bound tabular data, large |

Each group measures both **encode** (serialize) and **decode** (deserialize) operations, using `Throughput::Elements` for per-element metrics on scaled scenarios.

## Size Comparison Results

*From `cargo run --example size_report` on tealeaf-core:*

| Format | Small Object | 10K Points | 1K Users |
|--------|-------------|------------|----------|
| JSON | 1.00x | 1.00x | 1.00x |
| Protobuf | 0.38x | 0.65x | 0.41x |
| MessagePack | 0.35x | 0.63x | 0.38x |
| **TeaLeaf Binary** | 3.56x | **0.15x** | 0.47x |

**Key observations:**

- **Small objects:** TeaLeaf has a 64-byte header overhead. For objects under ~200 bytes, JSON or MessagePack are more compact.
- **Large arrays:** String deduplication and schema-based compression produce 6-7x better compression than JSON for 10K+ records.
- **Tabular data:** `@table` encoding with positional storage is competitive with Protobuf, with the advantage of embedded schemas.

## Speed Characteristics

TeaLeaf's dynamic key-based access is ~2-5x slower than Protobuf's generated code:

| Operation | TeaLeaf | Protobuf | JSON (serde) |
|-----------|---------|----------|--------------|
| Parse text | Moderate | N/A | Fast |
| Decode binary | Moderate | Fast | N/A |
| Random key access | O(1) hash | O(1) field | O(n) parse |

**Why TeaLeaf is slower than Protobuf:**

1. **Dynamic dispatch** -- fields resolved by name at runtime; Protobuf uses generated code with known offsets
2. **String table lookup** -- each string access requires a table lookup
3. **Schema resolution** -- schema structure parsed from binary at load time

**When this matters:**

- Hot loops decoding millions of records → consider Protobuf
- Cold reads or moderate throughput → TeaLeaf is fine
- Size-constrained transmission → TeaLeaf's smaller binary compensates for slower decode

## Code Structure

```
tealeaf-core/benches/
├── benchmarks.rs          # Entry point: criterion_group + criterion_main
├── common/
│   ├── mod.rs             # Module exports
│   ├── data.rs            # Test data generation functions
│   └── structs.rs         # Rust struct definitions (serde-compatible)
└── scenarios/
    ├── mod.rs             # Module exports
    ├── small_object.rs    # Small config object benchmarks
    ├── large_array.rs     # Scaled array benchmarks (100-10K)
    ├── nested_structs.rs  # Nesting depth benchmarks (2-100)
    ├── mixed_types.rs     # Heterogeneous data benchmarks
    └── tabular_data.rs    # @table User record benchmarks (100-5K)
```

Each scenario module exports `bench_encode` and `bench_decode` functions. Scaled scenarios accept a `size` parameter.

> For optimization tips and practical guidance on when to use each format, see [Performance](../guides/performance.md).
