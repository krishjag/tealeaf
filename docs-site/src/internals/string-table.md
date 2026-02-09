# String Table

The string table is a core component of the binary format that provides string deduplication.

## Purpose

In a typical document with 1,000 user records, field values like `"active"`, `"Engineering"`, or city names repeat frequently. Without deduplication, each occurrence stores the full string. The string table stores each unique string once and uses 4-byte indices everywhere else.

## Structure

```
┌─────────────────────────────┐
│ Total Size: u32              │  Size of the entire string table section
│ Count: u32                   │  Number of unique strings
├─────────────────────────────┤
│ Offsets: [u32 × Count]       │  Byte offset of each string in the data section
│ Lengths: [u32 × Count]       │  Length of each string (up to 4 GB)
├─────────────────────────────┤
│ String Data: [u8...]         │  Concatenated UTF-8 string data
└─────────────────────────────┘
```

## How It Works

### During Compilation

1. The writer traverses all values in the document
2. Every unique string is collected (keys, string values, schema names, field names, ref names, tag names)
3. Duplicates are eliminated
4. Each string gets an index (0, 1, 2, ...)
5. The string table is written first in the file
6. All subsequent encoding uses indices instead of raw strings

### During Reading

1. The reader loads the string table at startup
2. When decoding a string value, it reads a `u32` index
3. The index maps to an offset and length in the string data
4. The string is read from the data section

## Lookup Performance

String table access is **O(1)** by index:

```
index → offsets[index] → offset in data section
index → lengths[index] → number of bytes to read
string = data[offset..offset+length]
```

## Size Impact

### Example: 1,000 Users with 5 Fields

Without deduplication:
- Field names repeated 1,000 times each
- Common values ("active", "Engineering") repeated many times
- Estimated overhead: ~20-30 KB just for repeated strings

With string table:
- Each unique string stored once
- References are 4 bytes each
- Estimated savings: 60-80% on string data

### Extreme Case: Large Tabular Data

For 10,000 rows with 10 fields, field names alone would consume:

| Approach | Field Name Storage |
|----------|-------------------|
| JSON (per-field) | ~10 × 10,000 × avg(8 bytes) = ~800 KB |
| TeaLeaf (string table) | 10 × avg(8 bytes) + 100,000 × 4 bytes = ~400 KB |
| TeaLeaf with schema | 10 × avg(8 bytes) = ~80 bytes (field names in schema only!) |

With schema-typed data, field names appear only in the schema table -- the string table contains only the actual string values.

## What Gets Deduplicated

| String Source | Deduplicated? |
|---------------|---------------|
| Top-level key names | Yes |
| Object field names | Yes |
| String values | Yes |
| Schema names | Yes |
| Schema field names | Yes |
| Reference names | Yes |
| Tag names | Yes |

## Maximum String Length

String lengths are stored as `u32`, supporting individual strings up to ~4 GB. The total string table size (all strings + metadata) is also capped at `u32::MAX` by the table's `Size` header field.

## Interaction with Compression

The string table itself is not compressed (it's needed for decoding). However, data sections that reference the string table benefit doubly:
- String references are 4 bytes (already compact)
- ZLIB compression can further compress repetitive index patterns

## Implementation Note

The string table uses a `HashMap<String, u32>` during compilation for O(1) dedup lookups. The final table is written as parallel arrays (offsets + lengths + data) for O(1) indexed access during reading.
