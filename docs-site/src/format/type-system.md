# Type System

TeaLeaf has a rich type system covering primitives, containers, and type modifiers.

## Primitive Types

| Type | Aliases | Description | Binary Size |
|------|---------|-------------|-------------|
| `bool` | | true/false | 1 byte |
| `int8` | | Signed 8-bit integer | 1 byte |
| `int16` | | Signed 16-bit integer | 2 bytes |
| `int` | `int32` | Signed 32-bit integer | 4 bytes |
| `int64` | | Signed 64-bit integer | 8 bytes |
| `uint8` | | Unsigned 8-bit integer | 1 byte |
| `uint16` | | Unsigned 16-bit integer | 2 bytes |
| `uint` | `uint32` | Unsigned 32-bit integer | 4 bytes |
| `uint64` | | Unsigned 64-bit integer | 8 bytes |
| `float32` | | 32-bit IEEE 754 float | 4 bytes |
| `float` | `float64` | 64-bit IEEE 754 float | 8 bytes |
| `string` | | UTF-8 text | variable |
| `bytes` | | Raw binary data | variable |
| `json_number` | | Arbitrary-precision numeric string (from JSON) | variable |
| `timestamp` | | Unix milliseconds (i64) + timezone offset (i16) | 10 bytes |

## Type Modifiers

```tl
field: string          # required string
field: string?         # nullable string (can be ~ or null)
field: []string        # required array of strings
field: []string?       # nullable array of strings (the field itself can be ~ or null)
field: []user          # array of structs
```

The `?` modifier applies to the **field**, not array elements. However, the parser does accept `~` and `null` values inside arrays, including schema-typed arrays.

In `@table` tuples, `~` and `null` have distinct semantics for nullable fields: `~` means absent (field dropped from output), `null` means explicit null (always preserved). See [Schemas](schemas.md#nullable-fields) for details.

## Value Types (Not Schema Types)

The following are value types that appear in data but **cannot** be declared as field types in `@struct`:

| Type | Description |
|------|-------------|
| `object` | Untyped `{ key: value }` collections |
| `map` | Ordered `@map { key: value }` with any key type |
| `ref` | Reference (`!name`) to another value |
| `tagged` | Tagged value (`:tag value`) |

For structured fields, define a named struct and use it as the field type. For tagged values with a known set of variants, define a `@union` -- this provides schema metadata (variant names, field names, field types) that is preserved in the binary format.

## Type Widening

When reading binary data, automatic safe conversions apply:

- `int8` → `int16` → `int32` → `int64`
- `uint8` → `uint16` → `uint32` → `uint64`
- `float32` → `float64`

Narrowing conversions are not automatic and require recompilation.

## Type Inference

### Standalone Values

When writing, the smallest representation is selected:

- **Integers:** `i8` if fits, else `i16`, else `i32`, else `i64`
- **Unsigned:** `u8` if fits, else `u16`, else `u32`, else `u64`
- **Floats:** always `f64` at runtime

### Homogeneous Arrays

Arrays of uniform type use optimized encoding:

| Array Contents | Encoding Strategy |
|---|---|
| Schema-typed objects (matching a `@struct`) | Struct array encoding with null bitmaps |
| `Value::Int` arrays | Packed `Int32` encoding |
| `Value::String` arrays | String table indices (`u32`) |
| All other arrays (UInt, Float, Bool, mixed, etc.) | Heterogeneous encoding with per-element type tags |

## Type Coercion at Compile Time

When compiling schema-bound data, type mismatches use default values rather than erroring:

| Target Type | Mismatch Behavior |
|---|---|
| Numeric fields | Integers/floats coerce; non-numeric becomes `0` |
| String fields | Non-string becomes empty string `""` |
| Bytes fields | Non-bytes becomes empty bytes (length 0) |
| Timestamp fields | Non-timestamp becomes epoch (`0`) |

This "best effort" approach prioritizes successful compilation over strict validation. Validate at the application level before compilation for strict type checking.

## Bytes Literal

The text format supports `b"..."` hex literals for byte data:

```tl
payload: b"cafef00d"
empty: b""
checksum: b"CA FE"   # ERROR -- no spaces allowed
```

- Contents are hex digits only (uppercase or lowercase)
- Length must be even (2 hex chars per byte)
- `dumps()` and `decompile` emit `b"..."` for `Value::Bytes`, enabling full text round-trip
- JSON export encodes bytes as `"0xcafef00d"` strings; JSON import does not auto-convert back to bytes
