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

// Convert JSON to TeaLeaf document
string json = """{"name": "Alice", "age": 30}""";
using var doc = TLDocument.FromJson(json);

// Get the TeaLeaf text format (with schema definitions)
string tlText = doc.ToText();

// Convert back to JSON
string roundTrip = doc.ToJson();

// Compile to binary format for compact storage
doc.Compile("data.tlbx", compress: true);

// Read binary file
using var reader = TLReader.Open("data.tlbx");
string fromBinary = reader.ToJson();

// Access values by key
var name = reader.Get("name")?.AsString();
```

## Working with Text Format (.tl)

```csharp
using TeaLeaf;

// Parse TeaLeaf text
string tlText = """
    @struct Person (name: string, age: int)
    person: @Person ("Alice", 30)
    """;
using var doc = TLDocument.Parse(tlText);

// Access values
var person = doc.Get("person");
Console.WriteLine(person?.GetField("name")?.AsString()); // "Alice"

// Convert to JSON
Console.WriteLine(doc.ToJson());
```

## Working with Binary Format (.tlbx)

```csharp
using TeaLeaf;

// Create binary from JSON (one step)
TLReader.CreateFromJson(jsonString, "output.tlbx", compress: true);

// Or from a document
using var doc = TLDocument.FromJson(jsonString);
doc.Compile("output.tlbx", compress: true);

// Read binary file
using var reader = TLReader.Open("data.tlbx");

// Access data
foreach (var key in reader.Keys)
{
    Console.WriteLine($"{key}: {reader.GetAsJson(key)}");
}

// Memory-mapped reading for large files
using var mmapReader = TLReader.OpenMmap("large.tlbx");
```

## Supported Platforms

| Platform | Architecture |
|----------|--------------|
| Windows | x64, ARM64 |
| Linux | x64, ARM64 |
| macOS | x64 (Intel), ARM64 (Apple Silicon) |

## API Reference

### TLDocument

For working with TeaLeaf text format (.tl):

| Method | Description |
|--------|-------------|
| `TLDocument.Parse(string text)` | Parse TeaLeaf text |
| `TLDocument.ParseFile(string path)` | Parse from .tl file |
| `TLDocument.FromJson(string json)` | Create from JSON with schema inference |
| `doc.ToText()` | Convert to TeaLeaf text (with schemas) |
| `doc.ToJson()` | Convert to pretty JSON |
| `doc.ToJsonCompact()` | Convert to compact JSON |
| `doc.Compile(path, compress)` | Write to binary .tlbx file |
| `doc.Get(key)` | Get value by key |
| `doc.Keys` | Get all keys |

### TLReader

For reading binary TeaLeaf files (.tlbx):

| Method | Description |
|--------|-------------|
| `TLReader.Open(string path)` | Open binary file |
| `TLReader.OpenMmap(string path)` | Open with memory mapping |
| `TLReader.CreateFromJson(json, path, compress)` | Create binary from JSON |
| `reader.ToJson()` | Convert to pretty JSON |
| `reader.ToJsonCompact()` | Convert to compact JSON |
| `reader.Get(key)` | Get value by key |
| `reader.GetAsJson(key)` | Get value as JSON string |
| `reader.Keys` | Get all keys |
| `reader.Schemas` | Get schema definitions |

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

TeaLeaf text output (with auto-inferred schema):
```
@struct User (name: string, age: int)

users: @table User [
  ("Alice", 30)
  ("Bob", 25)
]
```

## License

MIT OR Apache-2.0

## Links

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [Documentation](https://github.com/krishjag/tealeaf#readme)
