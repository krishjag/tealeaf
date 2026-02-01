# compile

Compile a TeaLeaf text file (`.tl`) to the compact binary format (`.tlbx`).

## Usage

```bash
tealeaf compile <input.tl> -o <output.tlbx>
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<input.tl>` | Yes | Path to the TeaLeaf text file |
| `-o <output.tlbx>` | Yes | Path for the output binary file |

## Description

The `compile` command:

1. Parses the text file (including any `@include` directives)
2. Builds the string table (deduplicates all strings)
3. Encodes schemas into the schema table
4. Encodes each top-level key-value pair as a data section
5. Applies per-section ZLIB compression (enabled by default)
6. Writes the binary file with the 64-byte header

Compression is applied to sections larger than 64 bytes where the compressed size is less than 90% of the original.

## Examples

```bash
# Basic compilation
tealeaf compile config.tl -o config.tlbx

# Compile and inspect
tealeaf compile data.tl -o data.tlbx
tealeaf info data.tlbx
```

## Output

On success, prints compilation details including:
- Number of sections
- Number of schemas
- String table size
- Compression ratio
- Output file size

## Error Cases

| Error | Cause |
|-------|-------|
| Parse error | Invalid TeaLeaf syntax in input file |
| I/O error | Input file not found or output path not writable |
| Include error | Referenced `@include` file not found |

## See Also

- [`decompile`](./decompile.md) -- reverse operation
- [`validate`](./validate.md) -- check syntax without compiling
- [Binary Format](../format/binary-format.md) -- binary layout details
