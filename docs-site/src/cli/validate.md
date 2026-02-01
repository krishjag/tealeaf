# validate

Validate a TeaLeaf text file for syntactic correctness without compiling it.

## Usage

```bash
tealeaf validate <file.tl>
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<file.tl>` | Yes | Path to the TeaLeaf text file |

## Description

The `validate` command parses the text file and reports any syntax errors. It does not produce any output files.

Validation checks include:
- Lexical analysis (valid tokens, string escaping)
- Structural parsing (matched brackets, valid directives)
- Schema reference validity (`@table` references defined `@struct`)
- Include file resolution
- Type syntax in schema definitions

## Examples

```bash
# Validate a file
tealeaf validate config.tl

# Validate before compiling
tealeaf validate data.tl && tealeaf compile data.tl -o data.tlbx
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | File is valid |
| `1` | Validation errors found |

Error details are written to stderr with line/column information when available.

## See Also

- [`info`](./info.md) -- inspect file contents
- [`compile`](./compile.md) -- compile (implies validation)
