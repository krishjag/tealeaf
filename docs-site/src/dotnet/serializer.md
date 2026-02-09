# Reflection Serializer

The `TeaLeafSerializer` class provides runtime reflection-based serialization for scenarios where the source generator isn't suitable.

## When to Use

| Scenario | Approach |
|----------|----------|
| Known types at compile time | Source Generator (recommended) |
| Generic types (`T`) | Reflection Serializer |
| Types you don't control (third-party) | Reflection Serializer |
| Dynamic/runtime-determined types | Reflection Serializer |
| Maximum performance | Source Generator |

## API

All methods are on the static `TeaLeafSerializer` class.

### Serialization

```csharp
// To document text (schemas + data)
string docText = TeaLeafSerializer.ToDocument<User>(user);
string docText = TeaLeafSerializer.ToDocument<User>(user, key: "custom_key");

// To TeaLeaf text (data only, no schemas)
string text = TeaLeafSerializer.ToText<User>(user);

// To TLDocument (for further operations)
using var doc = TeaLeafSerializer.ToTLDocument<User>(user);
using var doc = TeaLeafSerializer.ToTLDocument<User>(user, key: "custom_key");

// To JSON (via native engine)
string json = TeaLeafSerializer.ToJson<User>(user);

// Compile to binary
TeaLeafSerializer.Compile<User>(user, "output.tlbx", compress: true);
```

### Deserialization

```csharp
// From TLDocument
using var doc = TLDocument.Parse(tlText);
var user = TeaLeafSerializer.FromDocument<User>(doc);
var user = TeaLeafSerializer.FromDocument<User>(doc, key: "custom_key");

// From TLValue (for nested types)
using var val = doc.Get("user");
var user = TeaLeafSerializer.FromValue<User>(val);

// From text
var user = TeaLeafSerializer.FromText<User>(tlText);
```

### Schema Generation

```csharp
// Get schema string
string schema = TeaLeafSerializer.GetSchema<User>();
// "@struct user (id: int, name: string, email: string?)"

// Get TeaLeaf type name for a C# type
string typeName = TeaLeafTextHelper.GetTLTypeName(typeof(int));    // "int"
string typeName = TeaLeafTextHelper.GetTLTypeName(typeof(long));   // "int64"
string typeName = TeaLeafTextHelper.GetTLTypeName(typeof(DateTime)); // "timestamp"
```

## Type Mapping

The reflection serializer uses `TeaLeafTextHelper.GetTLTypeName()` for type resolution:

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
| `Dictionary<string, T>` | `object` |
| Enum | `string` |
| `[TeaLeaf]` class | struct reference |

## Attributes

The reflection serializer respects the same attributes as the source generator:

- `[TeaLeaf]` / `[TeaLeaf("name")]` -- struct name
- `[TLKey("key")]` -- document key
- `[TLSkip]` -- skip property
- `[TLOptional]` -- nullable field
- `[TLRename("name")]` -- rename field
- `[TLType("type")]` -- override type

## Text Helpers

The `TeaLeafTextHelper` class provides utilities used by the serializer:

```csharp
// PascalCase to snake_case
TeaLeafTextHelper.ToSnakeCase("MyProperty"); // "my_property"

// String quoting
TeaLeafTextHelper.NeedsQuoting("hello world"); // true
TeaLeafTextHelper.QuoteIfNeeded("hello world"); // "\"hello world\""
TeaLeafTextHelper.EscapeString("line\nnewline"); // "line\\nnewline"

// Value formatting
var sb = new StringBuilder();
TeaLeafTextHelper.AppendValue(sb, 42, typeof(int)); // "42"
TeaLeafTextHelper.AppendValue(sb, null, typeof(string)); // "~"
```

## Performance Considerations

The reflection serializer uses `System.Reflection` at runtime, which is slower than the source generator approach. For hot paths or high-throughput scenarios, prefer the source generator.

However, the actual binary compilation and native operations are identical -- both approaches use the same native Rust library under the hood. The performance difference is only in the C# serialization/deserialization layer.
