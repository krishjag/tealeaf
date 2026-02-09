# Binary Format

The TeaLeaf binary format (`.tlbx`) is the compact, machine-efficient representation. This page documents the binary layout.

## Constants

| Constant | Value |
|----------|-------|
| Magic | `TLBX` (4 bytes, ASCII) |
| Version Major | `2` |
| Version Minor | `0` |
| Header Size | 64 bytes |

## File Structure

```
┌──────────────────┐
│ Header (64 B)    │
├──────────────────┤
│ String Table     │
├──────────────────┤
│ Schema Table     │
├──────────────────┤
│ Section Index    │
├──────────────────┤
│ Data Sections    │
└──────────────────┘
```

All multi-byte values are **little-endian**.

## Header (64 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | Magic | `TLBX` |
| 4 | 2 | Version Major | `2` |
| 6 | 2 | Version Minor | `0` |
| 8 | 4 | Flags | bit 0: compress (advisory), bit 1: root_array |
| 12 | 4 | Reserved | (unused) |
| 16 | 8 | String Table Offset | `u64` LE |
| 24 | 8 | Schema Table Offset | `u64` LE |
| 32 | 8 | Index Offset | `u64` LE |
| 40 | 8 | Data Offset | `u64` LE |
| 48 | 4 | String Count | `u32` LE |
| 52 | 4 | Schema Count | `u32` LE |
| 56 | 4 | Section Count | `u32` LE |
| 60 | 4 | Reserved | (for future checksum; currently 0) |

**Flag semantics:**
- **Bit 0 (COMPRESS):** Advisory. Indicates one or more sections use ZLIB (deflate) compression. Compression is determined per-section via the entry flags in the section index. This flag is a hint for tooling only.
- **Bit 1 (ROOT_ARRAY):** Indicates the source document was a root-level JSON array.

## String Table

All unique strings are deduplicated and stored once:

```
┌─────────────────────────┐
│ Size: u32               │
│ Count: u32              │
├─────────────────────────┤
│ Offsets: [u32 × Count]  │
│ Lengths: [u32 × Count]  │
├─────────────────────────┤
│ String Data (UTF-8)     │
└─────────────────────────┘
```

Strings are referenced by 32-bit index throughout the file. This provides:
- **Deduplication** -- `"Seattle"` stored once, even if used 1,000 times
- **Fast lookup** -- O(1) index-based access
- **Compact references** -- 4 bytes per reference instead of the full string

## Schema Table

The schema table stores both struct and union definitions:

```
┌──────────────────────────────────────┐
│ Size: u32                            │
│ Struct Count: u16                    │
│ Union Count: u16                     │
├──────────────────────────────────────┤
│ Struct Offsets: [u32 × struct_count] │
│ Struct Definitions                   │
├──────────────────────────────────────┤
│ Union Offsets: [u32 × union_count]   │
│ Union Definitions                    │
└──────────────────────────────────────┘
```

> **Backward compatibility:** The `Union Count` field at offset +6 was previously reserved (always 0). Old readers that ignore this field and only read `Struct Count` structs continue to work -- they simply skip the union data.

### Struct Definition

```
Schema:
  name_idx: u32      (string table index)
  field_count: u16
  flags: u16         (reserved)

  Field (repeated × field_count):
    name_idx: u32    (string table index)
    type: u8         (TLType code)
    flags: u8        (bit 0: nullable, bit 1: is_array)
    extra: u16       (type reference -- see below)
```

**Field `extra` values:**
- For `STRUCT` (0x22) fields: string table index of the struct type name (`0xFFFF` = untyped object)
- For `TAGGED` (0x31) fields: string table index of the union type name (`0xFFFF` = untyped tagged value)
- For all other field types: `0xFFFF`

### Union Definition

```
Union:
  name_idx: u32         (string table index)
  variant_count: u16
  flags: u16            (reserved)

  Variant (repeated × variant_count):
    name_idx: u32       (string table index)
    field_count: u16
    flags: u16          (reserved)

    Field (repeated × field_count):
      name_idx: u32     (string table index)
      type: u8          (TLType code)
      flags: u8         (bit 0: nullable, bit 1: is_array)
      extra: u16        (same semantics as struct field extra)
```

Each union variant uses the same 8-byte field entry format as struct fields.

## Type Codes

```
0x00  NULL        0x0A  FLOAT32     0x20  ARRAY      0x30  REF
0x01  BOOL        0x0B  FLOAT64     0x21  OBJECT     0x31  TAGGED
0x02  INT8        0x10  STRING      0x22  STRUCT     0x32  TIMESTAMP
0x03  INT16       0x11  BYTES       0x23  MAP
0x04  INT32       0x12  JSONNUMBER  0x24  TUPLE (reserved)
0x05  INT64
0x06  UINT8
0x07  UINT16
0x08  UINT32
0x09  UINT64
```

> `TUPLE` (0x24) is reserved but not currently emitted. Tuples in text are parsed as arrays.

> `JSONNUMBER` (0x12) stores arbitrary-precision numeric strings that exceed the range of i64, u64, or f64. Stored as a string table index, identical to STRING encoding.

## Section Index

Maps named sections to data locations:

```
┌─────────────────────────┐
│ Size: u32               │
│ Count: u32              │
├─────────────────────────┤
│ Entries (32 B each)     │
└─────────────────────────┘
```

Each entry (32 bytes):

| Field | Type | Description |
|-------|------|-------------|
| `key_idx` | `u32` | String table index for section name |
| `offset` | `u64` | Absolute file offset to data |
| `size` | `u32` | Compressed size in bytes |
| `uncompressed_size` | `u32` | Original size before compression |
| `schema_idx` | `u16` | Schema index (`0xFFFF` if none) |
| `type` | `u8` | TLType code |
| `flags` | `u8` | bit 0: compressed, bit 1: is_array |
| `item_count` | `u32` | Count for arrays/maps |
| `reserved` | `u32` | (future use) |

## Data Encoding

### Primitives

| Type | Encoding |
|------|----------|
| Null | 0 bytes |
| Bool | 1 byte (`0x00` or `0x01`) |
| Int8/UInt8 | 1 byte |
| Int16/UInt16 | 2 bytes, LE |
| Int32/UInt32 | 4 bytes, LE |
| Int64/UInt64 | 8 bytes, LE |
| Float32 | 4 bytes, IEEE 754 LE |
| Float64 | 8 bytes, IEEE 754 LE |
| String | `u32` index into string table |
| Bytes | varint length + raw bytes |
| Timestamp | `i64` Unix milliseconds (LE, 8 bytes) + `i16` timezone offset in minutes (LE, 2 bytes). Total: 10 bytes |

### Varint Encoding

Used for bytes length:
- Continuation bit (`0x80`) + 7 value bits
- Least-significant group first

### Arrays (Top-Level, Homogeneous)

For `Value::Int` (when all values fit in i32) or `Value::String` arrays:

```
Count: u32
Element Type: u8 (Int32 or String)
Elements: [packed data]
```

All other uniform-type arrays (UInt, Bool, Float, Timestamp, Int64) use heterogeneous encoding.

### Arrays (Top-Level, Heterogeneous)

For mixed-type arrays:

```
Count: u32
Element Type: 0xFF (marker)
Elements: [type: u8, data, type: u8, data, ...]
```

### Arrays (Schema-Typed Fields)

Array fields within `@struct` use homogeneous encoding for ANY element type:

```
Count: u32
Element Type: u8 (field's declared type)
Elements: [packed typed values]
```

### Objects

```
Field Count: u16
Fields: [
  key_idx: u32    (string table index)
  type: u8        (TLType code)
  data: [type-specific]
]
```

### Struct Arrays (Optimal Encoding)

```
Count: u32
Schema Index: u16
Null Bitmap Size: u16
Rows: [
  Null Bitmap: [u8 × bitmap_size]
  Values: [non-null field values only]
]
```

The null bitmap tracks which fields are null:
- Bit `i` set = field `i` is null
- Only non-null values are stored
- Bitmap size = `ceil((field_count + 7) / 8)`

### Maps

```
Count: u32
Entries: [
  key_type: u8
  key_data: [type-specific]
  value_type: u8
  value_data: [type-specific]
]
```

### References

```
name_idx: u32    (string table index for reference name)
```

### Tagged Values

```
tag_idx: u32     (string table index for tag name)
value_type: u8   (TLType code)
value_data: [type-specific]
```

## Compression

- **Algorithm:** ZLIB (deflate)
- **Threshold:** Compress if data > 64 bytes AND compressed < 90% of original
- **Granularity:** Per-section (each section compressed independently)
- **Flag:** Bit 0 of entry flags indicates compression
- **Decompression:** Readers check the flag and decompress transparently
