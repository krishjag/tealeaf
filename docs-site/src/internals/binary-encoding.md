# Binary Encoding Details

Deep dive into how values are encoded in the `.tlbx` binary format.

## Encoding Strategy

The encoder selects the encoding strategy based on value type and context:

### Top-Level Values

Each top-level key-value pair becomes a **section** in the binary file. The section's type code and flags determine how to decode it.

### Primitive Encoding

| Type | Encoding | Size |
|------|----------|------|
| Null | Nothing (type code alone) | 0 bytes |
| Bool | `0x00` or `0x01` | 1 byte |
| Int8 | Signed byte | 1 byte |
| Int16 | 2 bytes, little-endian | 2 bytes |
| Int32 | 4 bytes, little-endian | 4 bytes |
| Int64 | 8 bytes, little-endian | 8 bytes |
| UInt8-64 | Same as signed, unsigned | 1-8 bytes |
| Float32 | IEEE 754, little-endian | 4 bytes |
| Float64 | IEEE 754, little-endian | 8 bytes |
| String | `u32` string table index | 4 bytes |
| Bytes | varint length + raw data | variable |
| Timestamp | `i64` Unix ms, little-endian | 8 bytes |

### Integer Size Selection

The writer automatically selects the smallest representation:

```
Value::Int(42)      → Int8 (1 byte)     // fits in i8
Value::Int(1000)    → Int16 (2 bytes)   // fits in i16
Value::Int(100000)  → Int32 (4 bytes)   // fits in i32
Value::Int(5×10⁹)  → Int64 (8 bytes)   // needs i64
```

## Struct Array Encoding

The most optimized encoding path is for arrays of schema-typed objects:

```
┌─────────────────────┐
│ Count: u32           │  Number of rows
│ Schema Index: u16    │  Which schema these rows follow
│ Null Bitmap Size: u16│  Bytes per row for null tracking
├─────────────────────┤
│ Row 0:               │
│   Null Bitmap: [u8]  │  One bit per field (1 = null)
│   Field 0 data       │  Only if not null
│   Field 1 data       │  Only if not null
│   ...                │
├─────────────────────┤
│ Row 1:               │
│   Null Bitmap: [u8]  │
│   Field data...      │
├─────────────────────┤
│ ...                  │
└─────────────────────┘
```

### Null Bitmap

- Size: `ceil((field_count + 7) / 8)` bytes per row
- Bit `i` set = field `i` is null
- Only non-null fields have data written

For a schema with 5 fields, the bitmap is 1 byte. If bit 2 is set, field 2 is null and its data is skipped.

### Field Data

Each non-null field is encoded according to its schema type:
- Primitive types: fixed-size encoding
- String: `u32` string table index
- Nested struct: recursively encoded fields (with their own null bitmap)
- Array field: count + typed elements

## Homogeneous Array Encoding

### Integer Arrays

```
Count: u32
Element Type: 0x04 (Int32)
Elements: [i32 × Count]  — packed, no type tags
```

### String Arrays

```
Count: u32
Element Type: 0x10 (String)
Elements: [u32 × Count]  — string table indices
```

### Schema-Typed Field Arrays

Arrays within struct fields use homogeneous encoding for their declared type:

```
Count: u32
Element Type: u8 (from schema field type)
Elements: [packed data]
```

## Heterogeneous Array Encoding

For mixed-type arrays or types not optimized for homogeneous encoding:

```
Count: u32
Element Type: 0xFF (heterogeneous marker)
Elements: [
  type: u8, data,
  type: u8, data,
  ...
]
```

Each element carries its own type tag.

## Object Encoding

```
Field Count: u16
Fields: [
  key_idx: u32    (string table index)
  type: u8        (value type code)
  data: [...]     (type-specific encoding)
]
```

Objects are the untyped key-value container. Unlike struct arrays, each field carries its name and type.

## Map Encoding

```
Count: u32
Entries: [
  key_type: u8,    key_data: [...],
  value_type: u8,  value_data: [...],
]
```

Both keys and values carry type tags.

## Reference Encoding

```
name_idx: u32    (string table index for the reference name)
```

A reference is just a string table pointer to the target name.

## Tagged Value Encoding

```
tag_idx: u32     (string table index for the tag name)
value_type: u8   (type code of the inner value)
value_data: [...]  (type-specific encoding of the inner value)
```

## Varint Encoding

Used for bytes length:

```
Value: 300 (0x012C)
Encoded: 0xAC 0x02

Bit layout:
  0xAC = 1_0101100  → continuation bit set, value bits: 0101100 (44)
  0x02 = 0_0000010  → no continuation, value bits: 0000010 (2)

  Result: 44 + (2 << 7) = 44 + 256 = 300
```

- Continuation bit: `0x80` — if set, more bytes follow
- 7 value bits per byte
- Least-significant group first

## Compression

Applied per section:

1. Check if uncompressed size > 64 bytes
2. Compress with ZLIB (deflate)
3. If compressed size < 90% of original, use compressed version
4. Set compression flag in section index entry
5. Store both `size` (compressed) and `uncompressed_size` in the index
