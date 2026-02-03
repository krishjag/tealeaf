# TeaLeaf

Schema-aware data format with human-readable text and compact binary representation.

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

## DTO Serialization

Annotate your classes with `[TeaLeaf]` for reflection-based serialization:

```csharp
using TeaLeaf;
using TeaLeaf.Annotations;

[TeaLeaf]
public class Employee
{
    public string Name { get; set; } = "";
    public int Age { get; set; }

    [TLRename("dept")]
    public string Department { get; set; } = "";

    [TLOptional]
    public string? Email { get; set; }
}

// Serialize to TeaLeaf text (with @struct schema)
var emp = new Employee { Name = "Alice", Age = 30, Department = "Engineering" };
string tlText = TeaLeafSerializer.ToDocument(emp);

// Serialize collection to TeaLeaf
var employees = new List<Employee> { emp };
string tableText = TeaLeafSerializer.ToText(employees, "employees");

// Compile directly to binary
TeaLeafSerializer.Compile(emp, "employee.tlbx", compress: true);

// Deserialize from TeaLeaf text
var restored = TeaLeafSerializer.FromText<Employee>(tlText);

// Deserialize from TLDocument
using var doc = TLDocument.Parse(tlText);
var fromDoc = TeaLeafSerializer.FromDocument<Employee>(doc);
```

### Attributes

| Attribute | Description |
|-----------|-------------|
| `[TeaLeaf]` | Marks a class for TeaLeaf serialization |
| `[TLKey("name")]` | Sets the top-level document key |
| `[TLRename("name")]` | Overrides the field name in output |
| `[TLType("float64")]` | Overrides the TeaLeaf type in schema |
| `[TLOptional]` | Marks field as nullable in schema |
| `[TLSkip]` | Excludes property from serialization |

### Source Generator

For compile-time code generation (better performance than reflection), add the `TeaLeaf.Generators` package:

```bash
dotnet add package TeaLeaf.Generators
```

This generates `ToTeaLeafText()`, `FromTeaLeaf()`, and other methods at compile time for classes annotated with `[TeaLeaf]`.

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

MIT

## Links

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [Documentation](https://krishjag.github.io/tealeaf/)
- [.NET Guide](https://krishjag.github.io/tealeaf/dotnet/overview.html)
- [Source Generator Guide](https://krishjag.github.io/tealeaf/dotnet/source-generator.html)
