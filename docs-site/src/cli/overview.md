# CLI Overview

The `tealeaf` command-line tool provides all operations for working with TeaLeaf files.

## Usage

```bash
tealeaf <command> [options]
```

## Commands

| Command | Description |
|---------|-------------|
| [`compile`](./compile.md) | Compile text (`.tl`) to binary (`.tlbx`) |
| [`decompile`](./decompile.md) | Decompile binary (`.tlbx`) to text (`.tl`) |
| [`info`](./info.md) | Show file information (auto-detects format) |
| [`validate`](./validate.md) | Validate text format syntax |
| [`to-json`](./json-conversion.md) | Convert TeaLeaf text to JSON |
| [`from-json`](./json-conversion.md) | Convert JSON to TeaLeaf text |
| [`tlbx-to-json`](./binary-json-conversion.md) | Convert TeaLeaf binary to JSON |
| [`json-to-tlbx`](./binary-json-conversion.md) | Convert JSON to TeaLeaf binary |
| `help` | Show help text |

## Global Options

```bash
tealeaf --version    # Print version
tealeaf help         # Show usage
tealeaf -h           # Show usage
tealeaf --help       # Show usage
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error (parse error, I/O error, invalid arguments) |

Error messages are written to stderr. Data output goes to stdout (when no `-o` flag is specified).

## Quick Examples

```bash
# Full workflow
tealeaf validate data.tl
tealeaf compile data.tl -o data.tlbx
tealeaf info data.tlbx
tealeaf to-json data.tl -o data.json
tealeaf decompile data.tlbx -o recovered.tl

# JSON conversion
tealeaf from-json api_response.json -o structured.tl
tealeaf json-to-tlbx api_response.json -o compact.tlbx
tealeaf tlbx-to-json compact.tlbx -o exported.json
```
