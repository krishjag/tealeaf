# info

Display information about a TeaLeaf file. Auto-detects whether the file is text or binary format.

## Usage

```bash
tealeaf info <file>
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<file>` | Yes | Path to a `.tl` or `.tlbx` file |

## Description

The `info` command auto-detects the file format (by checking for the `TLBX` magic bytes) and displays relevant information.

### For Text Files (`.tl`)

- Number of top-level keys
- Key names
- Number of schema definitions
- Schema details (name, fields, types)

### For Binary Files (`.tlbx`)

- Version information
- File size
- Header details (offsets, counts)
- String table statistics (count, total size)
- Schema table details (names, field counts)
- Section index (key names, sizes, compression ratios)

## Examples

```bash
# Inspect a text file
tealeaf info config.tl

# Inspect a binary file
tealeaf info data.tlbx
```

## See Also

- [`validate`](./validate.md) -- check syntax validity
- [`compile`](./compile.md) -- compile to binary
