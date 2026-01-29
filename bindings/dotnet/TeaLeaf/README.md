# TeaLeaf

Schema-aware document format with human-readable text and compact binary representation.

## Features

- **Dual formats**: Human-readable text (`.tl`) and compact binary (`.tlbx`)
- **Schema-aware**: Automatic struct inference for uniform object arrays
- **Type-preserving**: Full JSON type fidelity including number types
- **High-performance**: Native Rust core with zero-copy parsing
- **Cross-platform**: Windows, Linux, macOS (x64 and ARM64)

## Installation

```bash
dotnet add package TeaLeaf
```

## Quick Start

```csharp
using TeaLeaf;

// Convert JSON to TeaLeaf text format
string json = """{"name": "Alice", "age": 30}""";
string tl = TeaLeafConverter.JsonToTL(json);

// Convert TeaLeaf back to JSON
string roundTrip = TeaLeafConverter.TLToJson(tl);

// Convert to binary format for compact storage
byte[] binary = TeaLeafConverter.JsonToTLBX(json);

// Convert binary back to JSON
string fromBinary = TeaLeafConverter.TLBXToJson(binary);
```

## Supported Platforms

| Platform | Architecture |
|----------|--------------|
| Windows | x64, ARM64 |
| Linux (glibc) | x64, ARM64 |
| Linux (musl/Alpine) | x64, ARM64 |
| macOS | x64 (Intel), ARM64 (Apple Silicon) |

## API Reference

### TeaLeafConverter

Static class for format conversions:

| Method | Description |
|--------|-------------|
| `JsonToTL(string json)` | Convert JSON to TeaLeaf text format |
| `TLToJson(string tl)` | Convert TeaLeaf text to JSON |
| `JsonToTLBX(string json)` | Convert JSON to TeaLeaf binary format |
| `TLBXToJson(byte[] binary)` | Convert TeaLeaf binary to JSON |
| `TLToTLBX(string tl)` | Convert TeaLeaf text to binary |
| `TLBXToTL(byte[] binary)` | Convert TeaLeaf binary to text |

## TeaLeaf Format Example

JSON input:
```json
{
  "users": [
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25}
  ]
}
```

TeaLeaf text output:
```
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

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [Documentation](https://github.com/krishjag/tealeaf#readme)
