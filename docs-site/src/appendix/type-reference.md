# Type Reference

Complete reference table for all TeaLeaf types, their text syntax, binary encoding, and language mappings.

## Primitive Types

| TeaLeaf Type | Text Syntax | Binary Code | Binary Size | Rust Type | C# Type |
|---|---|---|---|---|---|
| `bool` | `true` / `false` | `0x01` | 1 byte | `bool` | `bool` |
| `int8` | `42` | `0x02` | 1 byte | `i8` | `sbyte` |
| `int16` | `1000` | `0x03` | 2 bytes | `i16` | `short` |
| `int` / `int32` | `100000` | `0x04` | 4 bytes | `i32` | `int` |
| `int64` | `5000000000` | `0x05` | 8 bytes | `i64` | `long` |
| `uint8` | `255` | `0x06` | 1 byte | `u8` | `byte` |
| `uint16` | `65535` | `0x07` | 2 bytes | `u16` | `ushort` |
| `uint` / `uint32` | `100000` | `0x08` | 4 bytes | `u32` | `uint` |
| `uint64` | `18446744073709551615` | `0x09` | 8 bytes | `u64` | `ulong` |
| `float32` | `3.14` | `0x0A` | 4 bytes | `f32` | `float` |
| `float` / `float64` | `3.14` | `0x0B` | 8 bytes | `f64` | `double` |
| `string` | `"hello"` / `hello` | `0x10` | 4 bytes (index) | `String` | `string` |
| `bytes` | `b"cafef00d"` | `0x11` | varint + data | `Vec<u8>` | `byte[]` |
| `json_number` | *(from JSON)* | `0x12` | 4 bytes (index) | `String` | `string` |
| `timestamp` | `2024-01-15T10:30:00Z` | `0x32` | 10 bytes | `(i64, i16)` | `DateTimeOffset` |

## Special Types

| TeaLeaf Type | Text Syntax | Binary Code | Description |
|---|---|---|---|
| `null` | `~` | `0x00` | Null/missing value |

## Container Types

| TeaLeaf Type | Text Syntax | Binary Code | Description |
|---|---|---|---|
| Array | `[1, 2, 3]` | `0x20` | Ordered collection |
| Object | `{key: value}` | `0x21` | String-keyed map |
| Struct | `(val, val, ...)` in `@table` | `0x22` | Schema-typed record |
| Map | `@map {key: value}` | `0x23` | Any-keyed ordered map |
| Tuple | `(val, val, ...)` | `0x24` (reserved) | Currently parsed as array |

## Semantic Types

| TeaLeaf Type | Text Syntax | Binary Code | Description |
|---|---|---|---|
| Ref | `!name` | `0x30` | Named reference |
| Tagged | `:tag value` | `0x31` | Discriminated value |

## Type Modifiers

| Modifier | Syntax | Description |
|----------|--------|-------------|
| Nullable | `type?` | Field can be `~` (null) |
| Array | `[]type` | Array of the given type |
| Nullable array | `[]type?` | The field itself can be null |

## Type Widening Path

```
int8 → int16 → int32 → int64
uint8 → uint16 → uint32 → uint64
float32 → float64
```

Widening is automatic when reading binary data. Narrowing requires recompilation.

## JSON Mapping

| TeaLeaf Type | JSON Output | JSON Input |
|---|---|---|
| Null | `null` | `null` → Null |
| Bool | `true`/`false` | boolean → Bool |
| Int | number | integer → Int |
| UInt | number | large integer → UInt |
| Float | number | decimal → Float |
| String | `"text"` | string → String |
| Bytes | `"0xhex"` | *(not auto-detected)* |
| JsonNumber | number | large/precise number → JsonNumber |
| Timestamp | `"ISO 8601"` | *(not auto-detected)* |
| Array | `[...]` | array → Array |
| Object | `{...}` | object → Object |
| Map | `[[k,v],...]` | *(not auto-detected)* |
| Ref | `{"$ref":"name"}` | *(not auto-detected)* |
| Tagged | `{"$tag":"t","$value":v}` | *(not auto-detected)* |
