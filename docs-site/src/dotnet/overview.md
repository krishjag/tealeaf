# .NET Guide: Overview

TeaLeaf provides .NET bindings through a NuGet package that includes a C# source generator and a reflection-based serializer, both backed by the native Rust library via P/Invoke.

## Architecture

```
┌─────────────────────────────────────────────┐
│  Your .NET Application                      │
├─────────────────────┬───────────────────────┤
│  Source Generator    │  Reflection Serializer│
│  (compile-time)      │  (runtime)            │
├─────────────────────┴───────────────────────┤
│  TeaLeaf Managed Layer (TLDocument, TLValue)│
├─────────────────────────────────────────────┤
│  P/Invoke (NativeMethods.cs)                │
├─────────────────────────────────────────────┤
│  tealeaf_ffi.dll / .so / .dylib (Rust)      │
└─────────────────────────────────────────────┘
```

## Installation

```bash
dotnet add package TeaLeaf
```

The NuGet package includes:
- `TeaLeaf.Annotations` -- attributes (`[TeaLeaf]`, `[TLSkip]`, etc.)
- `TeaLeaf.Generators` -- C# incremental source generator
- `TeaLeaf` -- managed wrapper types and reflection serializer
- Native libraries for all supported platforms

## Two Serialization Approaches

### 1. Source Generator (Recommended)

Zero-reflection, compile-time code generation:

```csharp
[TeaLeaf]
public partial class User
{
    public int Id { get; set; }
    public string Name { get; set; } = "";
    [TLOptional] public string? Email { get; set; }
}

// Generated methods
string schema = User.GetTeaLeafSchema();
string text = user.ToTeaLeafText();
string json = user.ToTeaLeafJson();
user.CompileToTeaLeaf("user.tlbx");
var loaded = User.FromTeaLeaf(doc);
```

**Requirements:**
- Class must be `partial`
- Annotated with `[TeaLeaf]`
- Properties must have public getters (and setters for deserialization)

### 2. Reflection Serializer

For generic types, dynamic scenarios, or types you don't control:

```csharp
using var doc = TeaLeafSerializer.ToDocument(user);
string text = TeaLeafSerializer.ToText(user);
string json = TeaLeafSerializer.ToJson(user);
var loaded = TeaLeafSerializer.Deserialize<User>(doc);
```

## Core Types

### `TLDocument`

The in-memory document, wrapping a native handle:

```csharp
// Parse text
using var doc = TLDocument.Parse("name: alice\nage: 30");

// Load from file
using var doc = TLDocument.ParseFile("data.tl");

// From JSON
using var doc = TLDocument.FromJson(jsonString);

// Access values
string[] keys = doc.Keys;
using var value = doc["name"];

// Output
string text = doc.ToText();
string json = doc.ToJson();
doc.Compile("output.tlbx", compress: true);
```

### `TLValue`

Represents any TeaLeaf value with type-safe accessors:

```csharp
using var val = doc["users"];

// Type checking
TLType type = val.Type;
bool isNull = val.IsNull;

// Primitive access
bool? b = val.AsBool();
long? i = val.AsInt();
double? f = val.AsFloat();
string? s = val.AsString();
byte[]? bytes = val.AsBytes();
DateTimeOffset? ts = val.AsDateTime();

// Collection access
int len = val.ArrayLength;
using var elem = val[0];
using var field = val["name"];
string[] keys = val.ObjectKeys;

// Dynamic conversion
object? obj = val.ToObject();
```

### `TLReader`

Binary file reader with optional memory mapping:

```csharp
// Standard read
using var reader = TLReader.Open("data.tlbx");

// Memory-mapped (zero-copy for large files)
using var reader = TLReader.OpenMmap("data.tlbx");

// Access
string[] keys = reader.Keys;
using var val = reader["users"];

// Schema introspection
int schemaCount = reader.SchemaCount;
string name = reader.GetSchemaName(0);
```

## Next Steps

- [Source Generator](./source-generator.md) -- compile-time code generation in detail
- [Attributes Reference](./attributes.md) -- all available annotations
- [Reflection Serializer](./serializer.md) -- runtime serialization
- [Native Types](./native-types.md) -- `TLDocument`, `TLValue`, `TLReader` API
- [Diagnostics](./diagnostics.md) -- compiler warnings and errors
- [Platform Support](./platforms.md) -- supported runtimes and architectures
