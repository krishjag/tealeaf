# Pax

Schema-aware document format with human-readable text and compact binary representation.

## Features

- **Dual formats**: Human-readable text (`.pax`) and compact binary (`.paxb`)
- **Schema-aware**: Automatic struct inference for uniform object arrays
- **Type-preserving**: Full JSON type fidelity including number types
- **High-performance**: Native Rust core with zero-copy parsing
- **Cross-platform**: Windows, Linux, macOS (x64 and ARM64)

## Installation

```bash
dotnet add package Pax
```

## Quick Start

```csharp
using Pax;

// Convert JSON to PAX text format
string json = """{"name": "Alice", "age": 30}""";
string pax = PaxConverter.JsonToPax(json);

// Convert PAX back to JSON
string roundTrip = PaxConverter.PaxToJson(pax);

// Convert to binary format for compact storage
byte[] binary = PaxConverter.JsonToPaxBinary(json);

// Convert binary back to JSON
string fromBinary = PaxConverter.PaxBinaryToJson(binary);
```

## Supported Platforms

| Platform | Architecture |
|----------|--------------|
| Windows | x64, ARM64 |
| Linux (glibc) | x64, ARM64 |
| Linux (musl/Alpine) | x64, ARM64 |
| macOS | x64 (Intel), ARM64 (Apple Silicon) |

## API Reference

### PaxConverter

Static class for format conversions:

| Method | Description |
|--------|-------------|
| `JsonToPax(string json)` | Convert JSON to PAX text format |
| `PaxToJson(string pax)` | Convert PAX text to JSON |
| `JsonToPaxBinary(string json)` | Convert JSON to PAX binary format |
| `PaxBinaryToJson(byte[] binary)` | Convert PAX binary to JSON |
| `PaxToPaxBinary(string pax)` | Convert PAX text to binary |
| `PaxBinaryToPax(byte[] binary)` | Convert PAX binary to text |

## PAX Format Example

JSON input:
```json
{
  "users": [
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25}
  ]
}
```

PAX text output:
```pax
@struct User { name age }
{
  users: [User]
    ("Alice" 30)
    ("Bob" 25)
}
```

## License

MIT OR Apache-2.0

## Links

- [GitHub Repository](https://github.com/krishjag/pax)
- [Documentation](https://github.com/krishjag/pax#readme)
