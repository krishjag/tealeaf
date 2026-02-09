# to-json / from-json

Convert between TeaLeaf text format and JSON.

## to-json

Convert a TeaLeaf text file to JSON.

### Usage

```bash
tealeaf to-json <input.tl> [-o <output.json>]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<input.tl>` | Yes | Path to the TeaLeaf text file |
| `-o <output.json>` | No | Output file path. If omitted, writes to stdout |

### Examples

```bash
# Write to file
tealeaf to-json data.tl -o data.json

# Write to stdout
tealeaf to-json data.tl

# Pipe to another tool
tealeaf to-json data.tl | jq '.users'
```

### Output Format

The output is pretty-printed JSON. See [JSON Interoperability](../format/json-interop.md) for type mapping details.

---

## from-json

Convert a JSON file to TeaLeaf text format with automatic schema inference.

### Usage

```bash
tealeaf from-json <input.json> -o <output.tl>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<input.json>` | Yes | Path to the JSON file |
| `-o <output.tl>` | Yes | Path for the output TeaLeaf text file |

### Schema Inference

`from-json` automatically infers schemas from JSON arrays of uniform objects:

1. **Array Detection** -- identifies arrays where all elements are objects with identical keys
2. **Name Inference** -- singularizes the parent key name (`"users"` → `user` schema)
3. **Type Inference** -- determines field types across all items
4. **Nullable Detection** -- fields with any `null` become nullable (`string?`)
5. **Nested Schemas** -- creates schemas for nested uniform objects

### Examples

```bash
# Convert with schema inference
tealeaf from-json api_data.json -o structured.tl

# Full pipeline: JSON → TeaLeaf text → Binary
tealeaf from-json data.json -o data.tl
tealeaf compile data.tl -o data.tlbx
```

### Example: Schema Inference in Action

**Input (`employees.json`):**
```json
{
  "employees": [
    {"id": 1, "name": "Alice", "dept": "Engineering"},
    {"id": 2, "name": "Bob", "dept": "Design"}
  ]
}
```

**Output (`employees.tl`):**
```tl
@struct employee (dept: string, id: int, name: string)

employees: @table employee [
  ("Engineering", 1, "Alice"),
  ("Design", 2, "Bob"),
]
```

## See Also

- [`tlbx-to-json` / `json-to-tlbx`](./binary-json-conversion.md) -- binary format JSON conversion
- [JSON Interoperability](../format/json-interop.md) -- type mappings and round-trip details
