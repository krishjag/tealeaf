# JSON Interoperability

TeaLeaf provides built-in bidirectional JSON conversion for easy integration with existing tools and systems.

## JSON to TeaLeaf

### CLI

```bash
# JSON to TeaLeaf text (with automatic schema inference)
tealeaf from-json input.json -o output.tl

# JSON to TeaLeaf binary
tealeaf json-to-tlbx input.json -o output.tlbx
```

### Rust API

```rust
let doc = TeaLeaf::from_json(json_string)?;

// With automatic schema inference for arrays
let doc = TeaLeaf::from_json_with_schemas(json_string)?;
```

### .NET API

```csharp
using var doc = TLDocument.FromJson(jsonString);
```

### Type Mappings (JSON → TeaLeaf)

| JSON Type | TeaLeaf Type |
|-----------|--------------|
| `null` | Null |
| `true` / `false` | Bool |
| number (integer) | Int (or UInt if > `i64::MAX`) |
| number (decimal, finite f64) | Float |
| number (exceeds i64/u64/f64) | JsonNumber |
| string | String |
| array | Array |
| object | Object |

### Limitations

JSON import is "plain JSON only" -- it does not recognize the special JSON forms used for TeaLeaf export:

| JSON Form | Result |
|---|---|
| `{"$ref": "name"}` | Plain Object (not a Ref) |
| `{"$tag": "...", "$value": ...}` | Plain Object (not a Tagged) |
| `[[key, value], ...]` | Plain Array (not a Map) |
| ISO 8601 strings | Plain String (not a Timestamp) |

For full round-trip fidelity with these types, use binary format (`.tlbx`) or reconstruct programmatically.

## TeaLeaf to JSON

### CLI

```bash
# Text to JSON
tealeaf to-json input.tl -o output.json

# Binary to JSON
tealeaf tlbx-to-json input.tlbx -o output.json
```

Both commands write to stdout if `-o` is not specified.

### Rust API

```rust
let json = doc.to_json()?;         // pretty-printed
let json = doc.to_json_compact()?;  // minified
```

### .NET API

```csharp
string json = doc.ToJson();         // pretty-printed
string json = doc.ToJsonCompact();   // minified
```

### Type Mappings (TeaLeaf → JSON)

| TeaLeaf Type | JSON Representation |
|--------------|---------------------|
| Null | `null` |
| Bool | `true` / `false` |
| Int, UInt | number |
| Float | number |
| JsonNumber | number (parsed back to JSON number) |
| String | string |
| Bytes | string (hex with `0x` prefix) |
| Array | array |
| Object | object |
| Map | array of `[key, value]` pairs |
| Timestamp | string (ISO 8601) |
| Ref | `{"$ref": "name"}` |
| Tagged | `{"$tag": "tagname", "$value": value}` |

## Schema Inference

When converting JSON to TeaLeaf, the `from-json` command (and `from_json_with_schemas` API) can automatically infer schemas from arrays of uniform objects.

### How It Works

1. **Array Detection** -- identifies arrays of objects with identical field sets
2. **Name Inference** -- singularizes parent key names (`"products"` → `product` schema)
3. **Type Inference** -- determines field types across all array items
4. **Nullable Detection** -- fields with any `null` values become nullable (`string?`)
5. **Nested Schemas** -- creates separate schemas for nested objects within array elements

### Example

**Input JSON:**

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

**Inferred TeaLeaf output:**

```tl
@struct billing_address (city: string, street: string)
@struct customer (billing_address: billing_address, id: int, name: string)

customers: @table customer [
  (("Boston", "123 Main"), 1, "Alice"),
  (("Denver", "456 Oak"), 2, "Bob"),
]
```

### Nested Schema Inference

When array elements contain nested objects, TeaLeaf creates schemas for those nested objects if they have uniform structure across all items:

- Nested objects become their own `@struct` definitions
- Parent schemas reference nested schemas by name (not `object` type)
- Deeply nested objects are handled recursively

## Round-Trip Considerations

| Path | Fidelity |
|------|----------|
| `.tl` → `.json` → `.tl` | Lossy -- schemas, comments, refs, tags, timestamps, maps are simplified |
| `.tl` → `.tlbx` → `.tl` | Lossless for data (comments stripped) |
| `.tl` → `.tlbx` → `.json` | Same as `.tl` → `.json` |
| `.json` → `.tl` → `.json` | Generally lossless for JSON-native types |
| `.json` → `.tlbx` → `.json` | Generally lossless for JSON-native types |

For types that don't round-trip through JSON (Ref, Tagged, Map, Timestamp, Bytes), use the binary format for lossless storage.
