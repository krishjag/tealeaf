# Performance

Performance characteristics of TeaLeaf across different operations.

## Size Efficiency

### Benchmark Results

| Format | Small Object | 10K Points | 1K Users |
|--------|-------------|------------|----------|
| JSON | 1.00x | 1.00x | 1.00x |
| Protobuf | 0.38x | 0.65x | 0.41x |
| MessagePack | 0.35x | 0.63x | 0.38x |
| **TeaLeaf Binary** | 3.56x | **0.15x** | 0.47x |

### Analysis

- **Small objects:** TeaLeaf has a 64-byte header overhead. For objects under ~200 bytes, JSON or MessagePack are more compact.
- **Large arrays:** TeaLeaf's string deduplication and schema-based compression shine. For 10K+ records, TeaLeaf achieves 6-7x better compression than JSON.
- **Medium datasets (1K records):** TeaLeaf is competitive with Protobuf, with the advantage of embedded schemas.

### Where Size Matters Most

| Scenario | Recommendation |
|----------|---------------|
| < 100 bytes payload | Use MessagePack or raw JSON |
| 1-10 KB | TeaLeaf text or JSON (overhead amortized) |
| 10 KB - 1 MB | TeaLeaf binary with compression |
| > 1 MB | TeaLeaf binary with compression (best gains) |

## Parse/Decode Speed

TeaLeaf's dynamic key-based access is ~2-5x slower than Protobuf's generated code:

| Operation | TeaLeaf | Protobuf | JSON (serde) |
|-----------|---------|----------|--------------|
| Parse text | Moderate | N/A | Fast |
| Decode binary | Moderate | Fast | N/A |
| Random key access | O(1) hash | O(1) field | O(n) parse |
| Full iteration | Moderate | Fast | Fast |

### Why TeaLeaf Is Slower Than Protobuf

1. **Dynamic dispatch** — TeaLeaf resolves fields by name at runtime; Protobuf uses generated code with known offsets
2. **String table lookup** — each string access requires a table lookup
3. **Schema resolution** — schema structure is parsed from binary at load time

### When This Matters

- **Hot loops** decoding millions of records → consider Protobuf
- **Cold reads** or moderate throughput → TeaLeaf is fine
- **Size-constrained transmission** → TeaLeaf's smaller binary compensates for slower decode

## Memory-Mapped Reading

For large binary files, use memory-mapped I/O:

```rust
// Rust
let reader = Reader::open_mmap("large_file.tlbx")?;
```

```csharp
// .NET
using var reader = TLReader.OpenMmap("large_file.tlbx");
```

Benefits:
- **No upfront allocation** — data loaded on demand by the OS
- **Shared pages** — multiple processes can read the same file
- **Lazy loading** — only accessed sections are read from disk

## Compilation Performance

Compiling `.tl` to `.tlbx`:

| Input Size | Compile Time (approximate) |
|-----------|---------------------------|
| 1 KB | < 1 ms |
| 100 KB | ~10 ms |
| 1 MB | ~100 ms |
| 10 MB | ~1 second |

Compression adds ~20-50% to compile time but can reduce output size by 50-90%.

## Optimization Tips

### 1. Use Schemas for Tabular Data

Schema-bound `@table` data gets optimal encoding:
- Positional storage (no field name repetition)
- Null bitmaps (1 bit per nullable field vs full null markers)
- Type-homogeneous arrays

### 2. Enable Compression for Large Files

Compression is most effective for:
- Sections larger than 64 bytes
- Data with repeated string values
- Numeric arrays with patterns

```bash
tealeaf compile data.tl -o data.tlbx  # compression on by default
```

### 3. Use Binary Format for Storage

Text is for authoring; binary is for storage and transmission:

```
Text (.tl) → Author, review, version control
Binary (.tlbx) → Deploy, cache, transmit
```

### 4. Cache Compiled Binary

For data that's read frequently but written rarely:

```rust
// Compile once
doc.compile("cache.tlbx", true)?;

// Read many times (fast)
let reader = Reader::open_mmap("cache.tlbx")?;
```

### 5. Minimize String Diversity

String deduplication works best when values repeat:
- Enum-like fields (`"active"`, `"inactive"`) → deduplicated
- UUIDs or timestamps → each is unique, no deduplication benefit

### 6. Use the Right Integer Sizes

The writer auto-selects the smallest representation, but schema types guide encoding:

```tl
@struct sensor (
  id: uint16,       # 2 bytes instead of 4
  reading: float32, # 4 bytes instead of 8
  flags: uint8,     # 1 byte instead of 4
)
```
