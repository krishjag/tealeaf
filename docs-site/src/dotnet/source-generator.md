# Source Generator

The TeaLeaf source generator is a C# incremental source generator (`IIncrementalGenerator`) that generates serialization and deserialization code at compile time.

## How It Works

1. **Roslyn detects** classes annotated with `[TeaLeaf(Generate = true)]`
2. **ModelAnalyzer** examines the type's properties, attributes, and nested types
3. **TLTextEmitter** generates serialization methods
4. **DeserializerEmitter** generates deserialization methods
5. Generated code is added as a partial class extension

## Requirements

- Annotated with `[TeaLeaf(Generate = true)]` (from `TeaLeaf.Annotations`)
- The class must be `partial`
- Public properties with getters (and setters for deserialization)
- .NET 8.0+ with incremental source generator support

> **Note:** `[TeaLeaf]` without `Generate = true` is for reflection-based serialization only (`TeaLeafSerializer`) and does not require `partial`.

## Basic Example

```csharp
using TeaLeaf.Annotations;

[TeaLeaf(Generate = true)]
public partial class User
{
    public int Id { get; set; }
    public string Name { get; set; } = "";
    [TLOptional] public string? Email { get; set; }
    public bool Active { get; set; }
}
```

## Generated Methods

For each `[TeaLeaf(Generate = true)]` class, the generator produces:

### `GetTeaLeafSchema()`

Returns the `@struct` definition as a string:

```csharp
string schema = User.GetTeaLeafSchema();
// "@struct user (id: int, name: string, email: string?, active: bool)"
```

### `ToTeaLeafText()`

Serializes the instance to TeaLeaf text body format:

```csharp
string text = user.ToTeaLeafText();
// "(1, \"Alice\", \"alice@example.com\", true)"
```

### `ToTeaLeafDocument(string key = "user")`

Returns a complete TeaLeaf text document with schemas:

```csharp
string doc = user.ToTeaLeafDocument();
// "@struct user (id: int, name: string, email: string?, active: bool)\nuser: (1, ...)"
```

### `ToTLDocument(string key = "user")`

Parses through the native engine to create a `TLDocument`:

```csharp
using var doc = user.ToTLDocument();
string json = doc.ToJson();
doc.Compile("user.tlbx");
```

### `ToTeaLeafJson(string key = "user")`

Serializes to JSON via the native engine:

```csharp
string json = user.ToTeaLeafJson();
```

### `CompileToTeaLeaf(string path, string key = "user", bool compress = false)`

Compiles directly to a `.tlbx` binary file:

```csharp
user.CompileToTeaLeaf("user.tlbx", compress: true);
```

### `FromTeaLeaf(TLDocument doc, string key = "user")`

Deserializes from a `TLDocument`:

```csharp
using var doc = TLDocument.ParseFile("user.tlbx");
var loaded = User.FromTeaLeaf(doc);
```

### `FromTeaLeaf(TLValue value)`

Deserializes from a `TLValue` (for nested types):

```csharp
using var val = doc["user"];
var loaded = User.FromTeaLeaf(val);
```

## Nested Types

Types referencing other `[TeaLeaf]` types are fully supported:

```csharp
[TeaLeaf(Generate = true)]
public partial class Address
{
    public string Street { get; set; } = "";
    public string City { get; set; } = "";
}

[TeaLeaf(Generate = true)]
public partial class Person
{
    public string Name { get; set; } = "";
    public Address Home { get; set; } = new();
    [TLOptional] public Address? Work { get; set; }
}
```

Generated schema:
```tl
@struct address (street: string, city: string)
@struct person (name: string, home: address, work: address?)
```

## Collections

```csharp
[TeaLeaf(Generate = true)]
public partial class Team
{
    public string Name { get; set; } = "";
    public List<string> Tags { get; set; } = new();
    public List<Person> Members { get; set; } = new();
}
```

Generated schema:
```tl
@struct team (name: string, tags: []string, members: []person)
```

## Enum Support

Enums are serialized as snake_case strings:

```csharp
public enum Status { Active, Inactive, Suspended }

[TeaLeaf(Generate = true)]
public partial class User
{
    public string Name { get; set; } = "";
    public Status Status { get; set; }
}
```

In TeaLeaf text: `("Alice", active)`

## Type Mapping

| C# Type | TeaLeaf Type |
|---------|-------------|
| `bool` | `bool` |
| `int` | `int` |
| `long` | `int64` |
| `short` | `int16` |
| `sbyte` | `int8` |
| `uint` | `uint` |
| `ulong` | `uint64` |
| `ushort` | `uint16` |
| `byte` | `uint8` |
| `double` | `float` |
| `float` | `float32` |
| `decimal` | `float` |
| `string` | `string` |
| `DateTime` | `timestamp` |
| `DateTimeOffset` | `timestamp` |
| `byte[]` | `bytes` |
| `List<T>` | `[]T` |
| `T?` / `Nullable<T>` | `T?` |
| Enum | `string` |
| `[TeaLeaf]` class | struct reference |

## See Also

- [Attributes Reference](./attributes.md) -- all annotation options
- [Diagnostics](./diagnostics.md) -- compiler warnings (TL001-TL006)
