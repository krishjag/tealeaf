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
| `timestamp` | | Unix milliseconds (i64) | 8 bytes |

## Type Modifiers

```tl
field: string          # required string
field: string?         # nullable string (can be ~)
field: []string        # required array of strings
field: []string?       # nullable array of strings (the field itself can be ~)
field: []user          # array of structs
```

The `?` modifier applies to the **field**, not array elements. However, the parser does accept `~` (null) values inside arrays, including schema-typed arrays. Null elements are tracked in the null bitmap.

## Value Types (Not Schema Types)

The following are value types that appear in data but **cannot** be declared as field types in `@struct`:

| Type | Description |
|------|-------------|
| `object` | Untyped `{ key: value }` collections |
| `map` | Ordered `@map { key: value }` with any key type |
| `ref` | Reference (`!name`) to another value |
| `tagged` | Tagged value (`:tag value`) |

For structured fields, define a named struct and use it as the field type instead.

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

## Bytes Type Note

There is no dedicated bytes literal syntax in text format. The lexer parses `0x...` as integers, not bytes.

When bytes are serialized to text (via decompile), they are written as `0x...` hex strings, but these will parse back as integers — **bytes do not round-trip through text format**.

For bytes data:
- Use binary format (`.tlbx`) for lossless round-trips
- Construct `Value::Bytes` programmatically via the API
- JSON export encodes bytes as `"0xdeadbeef"` strings; JSON import does not auto-convert back to bytes
