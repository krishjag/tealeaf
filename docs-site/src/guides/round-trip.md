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

### Bytes → Hex Strings (JSON only)

Bytes round-trip losslessly within TeaLeaf text format using `b"..."` literals:

```tl
data: b"cafef00d"
```

However, JSON export converts bytes to hex strings:
```json
{"data": "0xcafef00d"}
```

Reimporting from JSON: becomes a plain string, not bytes.

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

Bytes now round-trip losslessly through text format using the `b"..."` literal:

```
Binary (bytes value) → Decompile → Text (b"..." literal) → Compile → Binary (bytes value)
```

The decompiler emits `b"cafef00d"` for bytes values, and the parser reads them back as `Value::Bytes`.

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

## Compact Floats: Intentional Lossy Optimization

The `--compact-floats` option (or `FormatOptions::compact().with_compact_floats()` in Rust) strips the `.0` suffix from whole-number floats to save characters and tokens:

```
# Default: preserves float type
revenue: 35934000000.0

# With compact_floats: saves 2 characters per whole-number float
revenue: 35934000000
```

**Trade-off:** Re-parsing the compact output produces `Value::Int` instead of `Value::Float` for these values. The numeric value is identical, but the type changes.

**When to use:**
- LLM context payloads where token savings matter more than type fidelity
- Financial datasets with many whole-number values (e.g., SEC EDGAR filings with `35934000000.0` → `35934000000`)
- Any scenario where downstream consumers treat int and float interchangeably

**When NOT to use:**
- When the distinction between `42` (int) and `42.0` (float) is semantically meaningful
- When data must survive multiple round-trips with identical types

**What's affected:**
- Only whole-number floats with ≤15 significant digits (e.g., `42.0`, `17164000000.0`)
- Non-whole floats are unaffected (`3.14` stays `3.14`)
- Special values are unaffected (`NaN`, `inf`, `-inf`)
- Very large floats using scientific notation are unaffected (`1e20` stays `1e20`)

### CLI Usage

```bash
# Compact whitespace + compact floats for maximum token savings
tealeaf from-json data.json -o data.tl --compact --compact-floats
tealeaf decompile data.tlbx -o data.tl --compact --compact-floats
```

### Rust API

```rust
use tealeaf::FormatOptions;

let opts = FormatOptions::compact().with_compact_floats();
let text = doc.to_tl_with_options(&opts);
```

### .NET API

```csharp
string text = doc.ToText(compact: true, compactFloats: true);
```

## Type Preservation Summary

| TeaLeaf Type | Binary Round-Trip | JSON Round-Trip |
|---|---|---|
| Null | Lossless | Lossless |
| Bool | Lossless | Lossless |
| Int | Lossless | Lossless |
| UInt | Lossless | Lossless (as number) |
| Float | Lossless | Lossless |
| Float (with `compact_floats`) | Lossless | Lossy (whole numbers → Int) |
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
