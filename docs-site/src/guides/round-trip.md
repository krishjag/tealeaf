# Round-Trip Fidelity

Understanding which conversion paths preserve data perfectly and where information can be lost.

## Round-Trip Matrix

| Path | Data Preserved | Lost |
|------|---------------|------|
| `.tl` → `.tlbx` → `.tl` | All data and schemas | Comments, formatting |
| `.tl` → `.json` → `.tl` | Basic types (string, number, bool, null, array, object) | Schemas, comments, refs, tags, maps, timestamps, bytes |
| `.tl` → `.tlbx` → `.json` | Same as `.tl` → `.json` | Same losses |
| `.json` → `.tl` → `.json` | All JSON-native types | (generally lossless) |
| `.json` → `.tlbx` → `.json` | All JSON-native types | (generally lossless) |
| `.tlbx` → `.tlbx` (recompile) | All data | (lossless) |

## Lossless: Text ↔ Binary

The text-to-binary-to-text round-trip preserves all data and schema information:

```bash
tealeaf compile original.tl -o compiled.tlbx
tealeaf decompile compiled.tlbx -o roundtrip.tl
tealeaf compile roundtrip.tl -o roundtrip.tlbx
# compiled.tlbx and roundtrip.tlbx contain equivalent data
```

**What's lost:**
- Comments (stripped during compilation)
- Whitespace and formatting
- The decompiled output may have different formatting than the original

**What's preserved:**
- All schemas (`@struct` definitions)
- All values (every type)
- Key ordering
- Schema-typed data (table structure)

## Lossy: TeaLeaf → JSON

JSON cannot represent all TeaLeaf types. The following conversions are one-way:

### Timestamps → Strings

```tl
created: 2024-01-15T10:30:00Z
```

JSON output:
```json
{"created": "2024-01-15T10:30:00.000Z"}
```

Reimporting: the ISO 8601 string becomes a plain `String`, not a `Timestamp`.

### Maps → Arrays

```tl
headers: @map {200: "OK", 404: "Not Found"}
```

JSON output:
```json
{"headers": [[200, "OK"], [404, "Not Found"]]}
```

Reimporting: becomes a plain nested array, not a `Map`.

### References → Objects

```tl
!ref: {x: 1, y: 2}
point: !ref
```

JSON output:
```json
{"point": {"$ref": "ref"}}
```

Reimporting: becomes a plain object with `$ref` key, not a `Ref`.

### Tagged Values → Objects

```tl
event: :click {x: 100, y: 200}
```

JSON output:
```json
{"event": {"$tag": "click", "$value": {"x": 100, "y": 200}}}
```

Reimporting: becomes a plain object, not a `Tagged`.

### Bytes → Hex Strings

```tl
# Bytes in binary format
data: <binary bytes value>
```

JSON output:
```json
{"data": "0xdeadbeef"}
```

Reimporting: becomes a plain string.

### Schemas → Lost

```tl
@struct user (id: int, name: string)
users: @table user [(1, alice), (2, bob)]
```

JSON output:
```json
{"users": [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}]}
```

The `@struct` definition is not represented in JSON. However, `from-json` can re-infer schemas from uniform arrays.

## Bytes and Text Format

Bytes have a unique round-trip issue even within TeaLeaf:

```
Binary (bytes value) → Decompile → Text (0x... hex) → Compile → Binary (integer value)
```

The decompiler writes bytes as hex integers, but the parser reads them back as integers. For lossless bytes round-trips, stay in binary format.

## Ensuring Lossless Round-Trips

### Use Binary for Storage

If you need to preserve all TeaLeaf types (refs, tags, maps, timestamps, bytes), keep data in `.tlbx`:

```bash
# Lossless cycle
tealeaf compile data.tl -o data.tlbx
tealeaf decompile data.tlbx -o data.tl
# data.tl preserves all types (except comments)
```

### Use JSON Only for Interop

JSON conversion is for integrating with JSON-based tools. Don't use it as a primary storage format if your data uses TeaLeaf-specific types.

### Verify with CLI

```bash
# Compile → JSON two ways, compare
tealeaf to-json data.tl -o from_text.json
tealeaf compile data.tl -o data.tlbx
tealeaf tlbx-to-json data.tlbx -o from_binary.json
# from_text.json and from_binary.json should be identical
```

## Type Preservation Summary

| TeaLeaf Type | Binary Round-Trip | JSON Round-Trip |
|---|---|---|
| Null | Lossless | Lossless |
| Bool | Lossless | Lossless |
| Int | Lossless | Lossless |
| UInt | Lossless | Lossless (as number) |
| Float | Lossless | Lossless |
| String | Lossless | Lossless |
| Bytes | Lossless | Lossy (→ hex string) |
| Array | Lossless | Lossless |
| Object | Lossless | Lossless |
| Map | Lossless | Lossy (→ array of pairs) |
| Ref | Lossless | Lossy (→ `$ref` object) |
| Tagged | Lossless | Lossy (→ `$tag`/`$value` object) |
| Timestamp | Lossless | Lossy (→ ISO 8601 string) |
| Schemas | Lossless | Lost (re-inferred on import) |
| Comments | Lost (stripped) | Lost |
