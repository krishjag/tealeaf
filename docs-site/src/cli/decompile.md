# decompile

Convert a TeaLeaf binary file (`.tlbx`) back to the human-readable text format (`.tl`).

## Usage

```bash
tealeaf decompile <input.tlbx> -o <output.tl> [--compact]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<input.tlbx>` | Yes | Path to the TeaLeaf binary file |
| `-o <output.tl>` | Yes | Path for the output text file |
| `--compact` | No | Remove insignificant whitespace for token-efficient output |

## Description

The `decompile` command:

1. Opens the binary file and reads the header
2. Loads the string table and schema table
3. Reads the section index
4. Decompresses sections as needed
5. Reconstructs `@struct` definitions from the schema table
6. Writes each section as a key-value pair in text format

## Notes

- **Comments are not preserved** -- comments from the original `.tl` are stripped during compilation
- **Formatting may differ** -- the decompiled output uses the default formatting, which may differ from the original source
- **Data is lossless** -- all values, schemas, and structure are preserved
- **Bytes are lossless** -- bytes values are written as `b"..."` hex literals, which round-trip correctly

## Compact Mode

The `--compact` flag removes insignificant whitespace from the output: no spaces after `:` in key-value pairs, no spaces after `,` in arrays and objects, no indentation, and no blank lines between sections. This produces a token-efficient format ideal for LLM context windows.

```bash
tealeaf decompile data.tlbx -o data_compact.tl --compact
```

The compact output is semantically identical to the pretty-printed output and round-trips without data loss.

## Examples

```bash
# Decompile a binary file
tealeaf decompile data.tlbx -o data_recovered.tl

# Decompile with compact output (fewer tokens for LLM input)
tealeaf decompile data.tlbx -o data_compact.tl --compact

# Round-trip verification
tealeaf compile original.tl -o compiled.tlbx
tealeaf decompile compiled.tlbx -o roundtrip.tl
tealeaf compile roundtrip.tl -o roundtrip.tlbx
# compiled.tlbx and roundtrip.tlbx should be equivalent
```

## See Also

- [`compile`](./compile.md) -- reverse operation
- [`info`](./info.md) -- inspect without decompiling
