# TeaLeaf Format Specification v2.0.0-beta.3

> **Status: Beta / Request for Comments**
>
> This specification is in beta stage. Feedback is welcome via GitHub issues.

This document defines the normative technical specification for the TeaLeaf format.

For an introduction, usage guide, API reference, and comparison with other formats, see the [README](../README.md).

---

## Table of Contents

1. [Text Format](#1-text-format)
   - [1.1 Comments](#11-comments)
   - [1.2 Strings](#12-strings)
   - [1.3 Numbers](#13-numbers)
   - [1.4 Boolean and Null](#14-boolean-and-null)
   - [1.5 Timestamps](#15-timestamps)
   - [1.6 Objects](#16-objects)
   - [1.7 Arrays](#17-arrays)
   - [1.8 Structs](#18-structs)
   - [1.9 Nested Structs](#19-nested-structs)
   - [1.10 Tables](#110-tables)
   - [1.11 Deep Nesting](#111-deep-nesting)
   - [1.12 Maps](#112-maps)
   - [1.13 References](#113-references)
   - [1.14 Tagged Values](#114-tagged-values)
   - [1.15 Unions](#115-unions)
   - [1.16 File Includes](#116-file-includes)
   - [1.17 Root Array](#117-root-array)
   - [1.18 Extensibility](#118-extensibility)
2. [Type System](#2-type-system)
   - [2.1 Primitive Types](#21-primitive-types)
   - [2.2 Type Modifiers](#22-type-modifiers)
   - [2.3 Type Widening](#23-type-widening)
   - [2.4 Type Inference](#24-type-inference)
   - [2.5 Type Coercion at Compile Time](#25-type-coercion-at-compile-time)
3. [Schema Compatibility](#3-schema-compatibility)
   - [3.1 Design Philosophy](#31-design-philosophy)
   - [3.2 Compatible Changes](#32-compatible-changes)
   - [3.3 Incompatible Changes](#33-incompatible-changes-require-recompile)
   - [3.4 Recompilation Workflow](#34-recompilation-workflow)
4. [Binary Format](#4-binary-format)
   - [4.1 Constants](#41-constants)
   - [4.2 File Structure](#42-file-structure)
   - [4.3 Header](#43-header-64-bytes)
   - [4.4 String Table](#44-string-table)
   - [4.5 Schema Table](#45-schema-table)
   - [4.6 Type Codes](#46-type-codes)
   - [4.7 Section Index](#47-section-index)
   - [4.8 Data Encoding](#48-data-encoding)
   - [4.9 Compression](#49-compression)
5. [Grammar](#5-grammar)
6. [JSON Interoperability](#6-json-interoperability)
   - [6.1 JSON to TeaLeaf](#61-json-to-tealeaf)
   - [6.2 TeaLeaf to JSON](#62-tealeaf-to-json)
   - [6.3 Schema Inference](#63-schema-inference)
7. [CLI Commands](#7-cli-commands)
8. [Error Types](#8-error-types)

---

## 1. Text Format

### 1.1 Comments

```tl
# This is a line comment
name: alice  # inline comment
```

Comments begin with `#` and extend to end of line.

### 1.2 Strings

**Simple strings** (unquoted, no whitespace or special characters):
```tl
name: alice
host: localhost
```

**Quoted strings** (with escape sequences):
```tl
greeting: "hello world"
path: "C:\\Users\\name"
message: "line1\nline2"
```

Escape sequences: `\\`, `\"`, `\n`, `\t`, `\r`, `\b` (backspace), `\f` (form feed), `\uXXXX` (Unicode code point, 4 hex digits)

**Multiline strings** (triple-quoted, auto-dedented):
```tl
description: """
  This is a multiline string.
  Leading whitespace is trimmed.
  Useful for documentation.
"""
```

### 1.3 Numbers

**Integers**:
```tl
count: 42
negative: -17
```

**Floats**:
```tl
price: 3.14
scientific: 6.022e23
```

**Hexadecimal**:
```tl
color: 0xFF5500
mask: 0x00A1
```

Both lowercase (`0x`, `0b`) and uppercase (`0X`, `0B`) prefixes are accepted.

Negative hex and binary literals are supported: `-0xFF`, `-0b1010`.

**Binary**:
```tl
flags: 0b1010
byte: 0b11110000
```

**Special float values**:
```tl
not_a_number: NaN
positive_infinity: inf
negative_infinity: -inf
```

These keywords represent IEEE 754 special values. In JSON export, `NaN` and infinity values are converted to `null`.

Numbers with exponent notation but no decimal point (e.g., `1e3`) are parsed as floats.

### 1.4 Boolean and Null

```tl
enabled: true
disabled: false
missing: ~
```

The tilde (`~`) represents null.

### 1.5 Timestamps

ISO 8601 formatted timestamps:

```tl
# Date only
created: 2024-01-15

# Date and time (UTC)
updated: 2024-01-15T10:30:00Z

# With milliseconds
precise: 2024-01-15T10:30:00.123Z

# With timezone offset
local: 2024-01-15T10:30:00+05:30
```

Format: `YYYY-MM-DD[THH:MM[:SS[.sss]][Z|+HH:MM|-HH:MM]]`

Seconds (`:SS`) are optional and default to `00` if omitted.

Timestamps are stored internally as Unix milliseconds (i64).

### 1.6 Objects

```tl
point: {x: 10, y: 20}

config: {
  host: localhost,
  port: 8080,
  debug: false,
}
```

Trailing commas are allowed.

### 1.7 Arrays

```tl
numbers: [1, 2, 3, 4, 5]
mixed: [1, "hello", true, ~]
nested: [[1, 2], [3, 4]]
```

### 1.8 Structs

Define a schema with `@struct` and use `@table` for schema-bound data:

```tl
@struct point (x: int, y: int)

# Use @table for schema-bound tuples
points: @table point [
  (0, 0),
  (100, 200),
]
```

**Important:** Standalone tuples (without `@table`) are parsed as plain arrays:

```tl
# This is an array [0, 0], NOT a point struct
origin: (0, 0)
```

**Optional type annotations:** Field types can be omitted and default to `string`:

```tl
@struct config (host, port: int, debug: bool)
# host defaults to string type
```

With types and nullable fields:

```tl
@struct user (
  id: int,
  name: string,
  email: string?,
  active: bool,
)

# Schema-bound table
users: @table user [
  (1, "Alice", "alice@example.com", true),
  (2, "Bob", ~, false),
]
```

### 1.9 Nested Structs

Structs can reference other structs. Nested tuples get schema binding from their parent field type:

```tl
@struct address (street: string, city: string, zip: string)

@struct person (
  name: string,
  home: address,
  work: address?,
)

# Use @table for schema-bound data
people: @table person [
  (
    "Alice Smith",
    ("123 Main St", "Berlin", "10115"),   # Parsed as address
    ("456 Office Blvd", "Berlin", "10117"), # Parsed as address
  ),
]
```

### 1.10 Tables

Tabular data with `@table`:

```tl
@struct user (id: int, name: string, email: string)

users: @table user [
  (1, alice, "alice@example.com"),
  (2, bob, "bob@example.com"),
  (3, carol, "carol@example.com"),
]
```

Tables provide optimal binary encoding with null bitmaps and positional storage.

### 1.11 Deep Nesting

```tl
@struct method (type: string, last_four: string)
@struct payment (amount: float, method: method)
@struct order (id: int, customer: string, payment: payment)

orders: @table order [
  (1, alice, (99.99, (credit, "4242"))),
  (2, bob, (49.50, (debit, "1234"))),
]
```

### 1.12 Maps

Dynamic key-value maps with the `@map` directive:

```tl
# String keys
headers: @map {
  "Content-Type": "application/json",
  "Accept": "*/*",
}

# Integer keys
status_codes: @map {
  200: "OK",
  404: "Not Found",
  500: "Internal Server Error",
}

# Mixed types
config: @map {
  name: "myapp",
  port: 8080,
  debug: true,
}
```

Maps preserve insertion order and support heterogeneous key types.

### 1.13 References

For graphs and deduplication:

```tl
# Define references
!node_a: {label: "Start", value: 1}
!node_b: {label: "End", value: 2}

# Use references
edges: [
  {from: !node_a, to: !node_b, weight: 1.0},
  {from: !node_b, to: !node_a, weight: 0.5},
]

# References can be used multiple times
nodes: [!node_a, !node_b]
```

### 1.14 Tagged Values

For discriminated unions:

```tl
events: [
  :click {x: 100, y: 200},
  :scroll {delta: -50},
  :keypress {key: "Enter"},
]
```

### 1.15 Unions

Discriminated unions with the `@union` directive:

```tl
@union shape {
  circle (radius: float),
  rectangle (width: float, height: float),
  point (),
}

shapes: [
  :circle (5.0),
  :rectangle (10.0, 20.0),
  :point (),
]
```

Union variants can have zero or more fields. Union definitions are encoded in the binary schema table alongside struct definitions, preserving variant names, field names, and field types through binary round-trips.

### 1.16 File Includes

Import other TeaLeaf files with `@include`:

```tl
# Include schemas from another file
@include "schemas/common.tl"

# Include shared configuration
@include "./shared/config.tl"

# Use included schemas
users: @table user [
  (1, alice),
]
```

Paths are resolved relative to the including file.

### 1.17 Root Array

The `@root-array` directive marks the document as representing a root-level JSON array rather than a JSON object. This is used for JSON round-trip fidelity.

```tl
@root-array

0: {id: 1, name: alice}
1: {id: 2, name: bob}
```

When present, JSON export (`to-json`, `tlbx-to-json`) produces a top-level JSON array `[{...}, {...}]` instead of a JSON object `{"0": {...}, "1": {...}}`.

The directive takes no arguments. It is emitted automatically by `from-json` and `json-to-tlbx` when the input JSON is a root-level array. In the binary format, the root-array flag is stored as bit 1 of the header flags field.

### 1.18 Extensibility

Unknown directives (e.g., `@custom`) at the document top level are silently ignored. If a same-line argument follows the directive (e.g., `@custom foo` or `@custom [1,2,3]`), it is consumed and discarded. Arguments on the next line are not consumed — they are parsed as normal statements. This enables forward compatibility: files authored for a newer spec version can be partially parsed by older implementations that do not recognize new directives.

When an unknown directive appears as a value (e.g., `key: @unknown [1,2,3]`), it is treated as `null`. The argument expression is consumed but discarded.

---

## 2. Type System

### 2.1 Primitive Types

| Type | Description | Binary Size |
|------|-------------|-------------|
| `bool` | true/false | 1 byte |
| `int8` | Signed 8-bit | 1 byte |
| `int16` | Signed 16-bit | 2 bytes |
| `int` / `int32` | Signed 32-bit | 4 bytes |
| `int64` | Signed 64-bit | 8 bytes |
| `uint8` | Unsigned 8-bit | 1 byte |
| `uint16` | Unsigned 16-bit | 2 bytes |
| `uint` / `uint32` | Unsigned 32-bit | 4 bytes |
| `uint64` | Unsigned 64-bit | 8 bytes |
| `float32` | 32-bit IEEE 754 | 4 bytes |
| `float` / `float64` | 64-bit IEEE 754 | 8 bytes |
| `string` | UTF-8 text | variable |
| `bytes` | Raw binary | variable |
| `timestamp` | Unix milliseconds + timezone offset | 10 bytes |

**Bytes literal:** The text format supports `b"..."` hex literals for byte data:

```
payload: b"cafef00d"
empty_bytes: b""
```

- Contents are hex digits only (uppercase or lowercase), no spaces
- Length must be even (2 hex chars per byte)
- Text serialization (`dumps`) emits `b"..."` for `Value::Bytes`
- JSON export encodes bytes as `"0xcafef00d"` strings; JSON import does not auto-convert these back to bytes

**Note:** `object`, `map`, `ref`, and `tagged` are value types, not schema types. They can appear in data but cannot be declared as field types in `@struct` definitions. For structured fields, define a named struct and use it as the field type. For tagged values with a known set of variants, define a `@union` to provide schema metadata that is preserved in the binary format.

### 2.2 Type Modifiers

```tl
field: string          # required string
field: string?         # nullable string (can be ~)
field: []string        # required array of strings
field: []string?       # nullable array of strings (field can be ~)
field: []user          # array of structs
```

**Note:** The `?` modifier applies to the field, not array elements. However, the parser does accept `~` (null) values inside arrays, including schema-typed arrays. Null elements are tracked in the null bitmap for struct arrays.

### 2.3 Type Widening

Automatic safe conversions when reading:
- `int8` → `int16` → `int32` → `int64`
- `uint8` → `uint16` → `uint32` → `uint64`
- `float32` → `float64`

### 2.4 Type Inference

**Standalone values:** When writing, the smallest representation is selected:
- Integers: i8 if fits, else i16, else i32, else i64
- Unsigned: u8 if fits, else u16, else u32, else u64
- Floats: always f64 at runtime

**Homogeneous arrays:** Arrays of uniform type use optimized encoding:
- Schema-typed arrays (objects matching a `@struct`): struct array encoding with null bitmaps
- `Value::Int` arrays where all values fit `i32`: packed Int32 encoding
- `Value::String` arrays: string table indices (u32)
- All other top-level arrays (including `Value::Int` exceeding i32, `Value::UInt`, `Value::Float`, `Value::Bool`, `Value::Timestamp`, mixed types): heterogeneous encoding with per-element type tags

### 2.5 Type Coercion at Compile Time

When compiling schema-bound data, type mismatches use default values rather than erroring:
- Numeric fields: integers/floats coerce; non-numeric becomes `0`
- String fields: non-string becomes empty string
- Bytes fields: non-bytes becomes empty bytes (length 0)
- Timestamp fields: non-timestamp becomes epoch (0)

This "best effort" approach prioritizes successful compilation over strict validation. For strict type checking, validate at the application level before compilation.

---

## 3. Schema Compatibility

### 3.1 Design Philosophy

TeaLeaf prioritizes simplicity over automatic schema evolution:

- **No migration machinery** — When schemas change, recompile the file
- **No version negotiation** — The embedded schema is the source of truth
- **Explicit over implicit** — Tuples require values for all fields

### 3.2 Compatible Changes

| Change | Notes |
|--------|-------|
| Rename field | Data is positional; names are documentation only |
| Widen type | int8 → int64, float32 → float64 (automatic) |

### 3.3 Incompatible Changes (Require Recompile)

| Change | Resolution |
|--------|-----------|
| Add field | Recompile source file |
| Remove field | Recompile source file |
| Reorder fields | Recompile source file |
| Narrow type | Recompile source file |

### 3.4 Recompilation Workflow

When schemas change:

```bash
tealeaf compile data.tl -o data.tlbx
```

The source `.tl` file is the master; regenerate binary as needed.

---

## 4. Binary Format

### 4.1 Constants

| Constant | Value |
|----------|-------|
| Magic | `TLBX` (4 bytes) |
| Version Major | 2 |
| Version Minor | 0 |
| Header Size | 64 bytes |

### 4.2 File Structure

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

### 4.3 Header (64 bytes)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | Magic | `TLBX` |
| 4 | 2 | Version Major | 2 |
| 6 | 2 | Version Minor | 0 |
| 8 | 4 | Flags | bit 0: compress (advisory), bit 1: root_array |
| 12 | 4 | Reserved | (unused) |
| 16 | 8 | String Table Offset | u64 LE |
| 24 | 8 | Schema Table Offset | u64 LE |
| 32 | 8 | Index Offset | u64 LE |
| 40 | 8 | Data Offset | u64 LE |
| 48 | 4 | String Count | u32 LE |
| 52 | 4 | Schema Count | u32 LE |
| 56 | 4 | Section Count | u32 LE |
| 60 | 4 | Reserved | (for future checksum; currently 0) |

All multi-byte values are little-endian.

**Flag semantics:**
- **Bit 0 (COMPRESS):** Advisory. Indicates one or more sections use ZLIB (deflate) compression. Compression is determined per-section via the entry flags in the section index (see §4.7). This header flag is a hint for tooling only.
- **Bit 1 (ROOT_ARRAY):** Indicates the source document was a root-level JSON array.

### 4.4 String Table

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

Strings are referenced by 32-bit index throughout the file.

### 4.5 Schema Table

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

**Backward compatibility:** The `Union Count` field at offset +6 was previously reserved (always 0). Old readers that ignore this field and only read `Struct Count` structs continue to work -- they simply skip the union data.

**Struct Definition:**

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

**Union Definition:**

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

### 4.6 Type Codes

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

> **Note:** `TUPLE` (0x24) is reserved but not currently emitted by the writer. Tuples in text format are parsed as arrays. The reader can decode this type code for forward compatibility.

> **Note:** `JSONNUMBER` (0x12) stores arbitrary-precision numeric strings that exceed the range of i64, u64, or f64. It is used internally to preserve exact decimal representation during JSON round-trips (e.g., integers larger than `u64::MAX` or floats that overflow `f64`). The value is stored as a string table index, identical to `STRING` encoding. In the text format, `JSONNUMBER` values are written as bare numeric literals. Through FFI, `JSONNUMBER` is transparent — it reports as `String` type and is accessible via string accessors.

### 4.7 Section Index

Maps named sections to data locations:

```
┌─────────────────────────┐
│ Size: u32               │
│ Count: u32              │
├─────────────────────────┤
│ Entries (32 B each)     │
└─────────────────────────┘

Entry (32 bytes):
  key_idx: u32           (string table index)
  offset: u64            (absolute file offset)
  size: u32              (compressed size)
  uncompressed_size: u32 (original size)
  schema_idx: u16        (0xFFFF if none)
  type: u8               (TLType code)
  flags: u8              (bit 0: compressed, bit 1: is_array)
  item_count: u32        (count for arrays/maps)
  reserved: u32
```

### 4.8 Data Encoding

**Primitives:**
- `Null`: 0 bytes
- `Bool`: 1 byte (0x00 or 0x01)
- `IntN/UIntN`: N/8 bytes, little-endian
- `FloatN`: N/8 bytes, IEEE 754 little-endian
- `String`: u32 index into string table
- `Bytes`: varint length + raw bytes
- `Timestamp`: i64 Unix milliseconds (LE, 8 bytes) + i16 timezone offset in minutes (LE, 2 bytes). Total: 10 bytes. Offset 0 = UTC. Positive = east of UTC, negative = west.

**Varint encoding** (for bytes length):
- Continuation bit (0x80) + 7 value bits
- Least-significant group first

**Arrays (top-level, homogeneous):**

For top-level arrays of `Value::Int` or `Value::String`:
```
Count: u32
Element Type: u8 (Int32 or String)
Elements: [packed data]
```

**Arrays (top-level, heterogeneous):**

For top-level arrays of other types (Float, Bool, UInt, Timestamp, mixed, etc.):
```
Count: u32
Element Type: 0xFF (marker)
Elements: [type: u8, data, type: u8, data, ...]
```

**Arrays (schema-typed fields):**

Array fields within schema-typed data use homogeneous encoding for ANY element type:
```
Count: u32
Element Type: u8 (field's declared type)
Elements: [packed typed values]
```
This applies to `[]int`, `[]float`, `[]bool`, `[]user`, etc. within `@struct` definitions.

**Objects:**
```
Field Count: u16
Fields: [
  key_idx: u32    (string table index)
  type: u8        (TLType code)
  data: [type-specific]
]
```

**Struct Arrays (optimal encoding):**
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
- Bit i set = field i is null
- Only non-null values are stored in the data section
- Bitmap size = ceil((field_count + 7) / 8)

**Maps:**
```
Count: u32
Entries: [
  key_type: u8
  key_data: [type-specific]
  value_type: u8
  value_data: [type-specific]
]
```

**References:**
```
name_idx: u32    (string table index for reference name)
```

**Tagged Values:**
```
tag_idx: u32     (string table index for tag name)
value_type: u8   (TLType code)
value_data: [type-specific]
```

**Tuples:**

Tuples in text format (`(a, b, c)`) are parsed as arrays. In binary format, they are encoded as arrays—the `TUPLE` type code (0x24) is reserved but not currently used by the writer.

### 4.9 Compression

- **Algorithm**: ZLIB (deflate)
- **Threshold**: Compress if data > 64 bytes AND compressed < 90% of original
- **Per-section**: Each section compressed independently
- **Flag**: Bit 0 of entry flags indicates compression

---

## 5. Grammar

```ebnf
document     = { directive | pair | ref_def } ;

directive    = struct_def | union_def | include | root_array ;
struct_def   = "@struct" name "(" fields ")" ;
union_def    = "@union" name "{" variants "}" ;
include      = "@include" string ;
root_array   = "@root-array" ;

variants     = variant { "," variant } ;
variant      = name "(" [ fields ] ")" ;

fields       = field { "," field } ;
field        = name [ ":" type ] ;  (* type defaults to string if omitted *)
type         = [ "[]" ] base_type [ "?" ] ;
base_type    = "bool" | "int" | "int8" | "int16" | "int32" | "int64"
             | "uint" | "uint8" | "uint16" | "uint32" | "uint64"
             | "float" | "float32" | "float64" | "string" | "bytes"
             | "timestamp" | name ;

pair         = key ":" value ;
key          = name | string ;
value        = primitive | object | array | tuple | table | map
             | tagged | ref | timestamp ;

primitive    = string | bytes_lit | number | bool | "~" ;
bytes_lit    = "b\"" { hex_digit hex_digit } "\"" ;
hex_digit    = "0"-"9" | "a"-"f" | "A"-"F" ;
object       = "{" [ ( pair | ref_def ) { "," ( pair | ref_def ) } ] "}" ;
array        = "[" [ value { "," value } ] "]" ;
tuple        = "(" [ value { "," value } ] ")" ;
table        = "@table" name array ;
map          = "@map" "{" [ map_entry { "," map_entry } ] "}" ;
map_entry    = map_key ":" value ;
map_key      = string | name | integer ;  (* restricted to hashable types *)
tagged       = ":" name value ;
ref          = "!" name ;                (* reference usage; definitions use ref_def *)
ref_def      = "!" name ":" value ;      (* reference definition at top-level or in objects *)
timestamp    = date [ "T" time [ timezone ] ] ;

date         = digit{4} "-" digit{2} "-" digit{2} ;
time         = digit{2} ":" digit{2} [ ":" digit{2} [ "." digit{1,3} ] ] ;
timezone     = "Z" | ( "+" | "-" ) digit{2} [ ":" ] digit{2}
             | ( "+" | "-" ) digit{2} ;  (* hour-only offset, minutes default to 00 *)

string       = name | '"' chars '"' | '"""' multiline '"""' ;
number       = integer | float | hex | binary ;
integer      = [ "-" ] digit+ ;
float        = [ "-" ] digit+ "." digit+ [ ("e"|"E") ["+"|"-"] digit+ ]
             | [ "-" ] digit+ ("e"|"E") ["+"|"-"] digit+
             | "NaN" | "inf" | "-inf" ;
hex          = [ "-" ] ("0x" | "0X") hexdigit+ ;
binary       = [ "-" ] ("0b" | "0B") ("0"|"1")+ ;
bool         = "true" | "false" ;
name         = (letter | "_") { letter | digit | "_" | "-" | "." } ;
comment      = "#" { any } newline ;

chars        = { any_char | escape } ;
escape       = "\\" | "\\\"" | "\\n" | "\\t" | "\\r" | "\\b" | "\\f"
             | "\\u" hexdigit hexdigit hexdigit hexdigit ;
```

---

## 6. JSON Interoperability

### 6.1 JSON to TeaLeaf

```rust
// Rust API
let doc = TeaLeaf::from_json(json_string)?;
```

```csharp
// .NET API
var doc = TLDocument.FromJson(jsonString);
```

Type mappings:
| JSON Type | TeaLeaf Type |
|-----------|--------------|
| null | Null |
| boolean | Bool |
| number (integer) | Int (or UInt if > i64::MAX) |
| number (decimal, finite f64) | Float |
| number (exceeds i64/u64/f64) | JsonNumber |
| string | String |
| array | Array |
| object | Object |

**Limitation:** JSON import is "plain JSON only" — it does not recognize the special JSON forms used for TeaLeaf export:
- `{"$ref": "name"}` becomes a plain Object, not a Ref
- `{"$tag": "...", "$value": ...}` becomes a plain Object, not a Tagged
- `[[key, value], ...]` becomes a plain Array, not a Map
- ISO 8601 strings become plain Strings, not Timestamps

For full round-trip fidelity with these types, use binary format (`.tlbx`) or reconstruct programmatically.

### 6.2 TeaLeaf to JSON

```rust
// Rust API
let json = doc.to_json()?;        // pretty-printed
let json = doc.to_json_compact()?; // minified
```

```csharp
// .NET API
string json = doc.ToJson();        // pretty-printed
string json = doc.ToJsonCompact(); // minified
```

Type mappings:
| TeaLeaf Type | JSON Type |
|--------------|-----------|
| Null | null |
| Bool | boolean |
| Int, UInt | number |
| Float | number |
| JsonNumber | number (parsed back to JSON number) |
| String | string |
| Bytes | string (hex, 0x prefix) |
| Array | array |
| Object | object |
| Map | array of `[key, value]` pairs |
| Timestamp | string (ISO 8601) |
| Ref | `{"$ref": "name"}` |
| Tagged | `{"$tag": "tagname", "$value": value}` |

### 6.3 Schema Inference

TeaLeaf can automatically infer schemas from JSON arrays of uniform objects:

```rust
// Rust API - with automatic schema inference
let doc = TeaLeaf::from_json_with_schemas(json_string)?;
let tl_text = doc.to_tl_with_schemas();
```

**How It Works:**

1. **Array Detection**: Identifies arrays of objects with identical field sets
2. **Name Inference**: Singularizes parent key names (`"products"` → `product` schema)
3. **Type Inference**: Determines field types across all array items
4. **Nullable Detection**: Fields with any `null` values become nullable (`string?`)
5. **Nested Object Schemas**: Creates separate schemas for nested objects within array elements

**Example:**

Input JSON:
```json
{
  "customers": [
    {
      "id": 1,
      "name": "Alice",
      "billing_address": {"street": "123 Main", "city": "Boston"}
    },
    {
      "id": 2,
      "name": "Bob",
      "billing_address": {"street": "456 Oak", "city": "Denver"}
    }
  ]
}
```

Inferred TeaLeaf output:
```tl
@struct billing_address (city: string, street: string)
@struct customer (billing_address: billing_address, id: int, name: string)

customers: @table customer [
  ((Boston, "123 Main"), 1, Alice),
  ((Denver, "456 Oak"), 2, Bob)
]
```

**Nested Schema Inference:**

When array elements contain nested objects, TeaLeaf creates schemas for those nested objects if they have uniform structure across all array items:

- Nested objects become their own `@struct` definitions
- Parent schemas reference nested schemas by name (not `object` type)
- Deeply nested objects are handled recursively

---

## 7. CLI Commands

```
tealeaf <command> [options]

Commands:
  compile <input.tl> -o <output.tlbx>       Compile text to binary
  decompile <input.tlbx> -o <output.tl>     Decompile binary to text
  info <file.tl|file.tlbx>                  Show file info (auto-detects format)
  validate <file.tl>                        Validate text format

JSON Conversion:
  to-json <input.tl> [-o <output.json>]     Convert TeaLeaf text to JSON
  from-json <input.json> -o <output.tl>     Convert JSON to TeaLeaf text (with schema inference)
  tlbx-to-json <input.tlbx> [-o <out.json>] Convert TeaLeaf binary to JSON
  json-to-tlbx <input.json> -o <out.tlbx>   Convert JSON to TeaLeaf binary

  help                                      Show help
```

**Notes:**
- `from-json` automatically infers schemas from uniform arrays
- `info` auto-detects whether file is text or binary format
- `compile` enables compression by default

---

## 8. Error Types

| Error | Description |
|-------|-------------|
| `Io` | File I/O error |
| `InvalidMagic` | Bad magic bytes in binary file |
| `InvalidVersion` | Unsupported version |
| `InvalidType` | Unknown type code |
| `InvalidUtf8` | String encoding error |
| `UnexpectedToken` | Parse error (expected vs got) |
| `UnexpectedEof` | Premature end of input |
| `UnknownStruct` | Schema not found |
| `MissingField` | Required field not provided |
| `ParseError` | Generic parse error |

---

*TeaLeaf Format Specification v2.0-beta.2*
