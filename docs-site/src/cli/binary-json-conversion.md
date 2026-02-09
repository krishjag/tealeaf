# tlbx-to-json / json-to-tlbx

Convert between TeaLeaf binary format and JSON directly, without going through the text format.

## tlbx-to-json

Convert a TeaLeaf binary file to JSON.

### Usage

```bash
tealeaf tlbx-to-json <input.tlbx> [-o <output.json>]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<input.tlbx>` | Yes | Path to the TeaLeaf binary file |
| `-o <output.json>` | No | Output file path. If omitted, writes to stdout |

### Examples

```bash
# Write to file
tealeaf tlbx-to-json data.tlbx -o data.json

# Write to stdout
tealeaf tlbx-to-json data.tlbx

# Pipe to jq for filtering
tealeaf tlbx-to-json data.tlbx | jq '.config'
```

### Notes

- Produces the same JSON output as `to-json` on the equivalent text file
- Reads the binary directly -- no intermediate text conversion

---

## json-to-tlbx

Convert a JSON file directly to TeaLeaf binary format.

### Usage

```bash
tealeaf json-to-tlbx <input.json> -o <output.tlbx>
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<input.json>` | Yes | Path to the JSON file |
| `-o <output.tlbx>` | Yes | Path for the output binary file |

### Examples

```bash
# Direct JSON to binary
tealeaf json-to-tlbx api_data.json -o compact.tlbx

# Verify the result
tealeaf info compact.tlbx
tealeaf tlbx-to-json compact.tlbx -o verify.json
```

### Notes

- Performs schema inference (same as `from-json`)
- Compiles directly to binary -- no intermediate `.tl` file
- Compression is enabled by default

## Workflow Comparison

```
# Two-step (via text)
tealeaf from-json data.json -o data.tl
tealeaf compile data.tl -o data.tlbx

# One-step (direct)
tealeaf json-to-tlbx data.json -o data.tlbx
```

Both approaches produce equivalent binary output.

## See Also

- [`to-json` / `from-json`](./json-conversion.md) -- text format JSON conversion
- [JSON Interoperability](../format/json-interop.md) -- type mappings and limitations
