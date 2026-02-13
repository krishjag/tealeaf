# TeaLeaf.Annotations

Attribute definitions for the TeaLeaf serialization framework. Use these attributes to annotate your C# classes and records for TeaLeaf format conversion.

## Installation

```bash
dotnet add package TeaLeaf.Annotations
```

> **Note**: This package is automatically included as a dependency of the main [TeaLeaf](https://www.nuget.org/packages/TeaLeaf) package. Install it directly only if you need the attributes without the runtime library.

## Attributes

| Attribute | Target | Description |
|-----------|--------|-------------|
| `[TeaLeaf]` | Class/Record | Marks a type for TeaLeaf serialization |
| `[TeaLeaf(Generate = true)]` | Class/Record | Enables compile-time source generation (requires `partial`) |
| `[TLKey("name")]` | Class/Record | Sets the top-level document key |
| `[TLRename("name")]` | Property | Overrides the field name in output |
| `[TLType("float64")]` | Property | Overrides the TeaLeaf type in schema |
| `[TLOptional]` | Property | Marks field as nullable in schema |
| `[TLSkip]` | Property | Excludes property from serialization |

## Usage

### Reflection-based serialization

```csharp
using TeaLeaf.Annotations;

[TeaLeaf]
[TLKey("employee")]
public class Employee
{
    public string Name { get; set; } = "";
    public int Age { get; set; }

    [TLRename("dept")]
    public string Department { get; set; } = "";

    [TLOptional]
    public string? Email { get; set; }

    [TLSkip]
    public string InternalId { get; set; } = "";
}
```

### Source generator (compile-time)

Add `Generate = true` and the `partial` keyword:

```csharp
using TeaLeaf.Annotations;

[TeaLeaf(Generate = true)]
[TLKey("employee")]
public partial class Employee
{
    public string Name { get; set; } = "";
    public int Age { get; set; }

    [TLRename("dept")]
    public string Department { get; set; } = "";

    [TLOptional]
    public string? Email { get; set; }

    [TLSkip]
    public string InternalId { get; set; } = "";
}
```

These attributes are consumed by:
- **TeaLeaf** runtime library (reflection-based serialization via `TeaLeafSerializer`)
- **TeaLeaf.Generators** source generator (compile-time code generation, requires `Generate = true`)

## License

MIT

## Links

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [.NET Guide](https://krishjag.github.io/tealeaf/dotnet/overview.html)
- [Source Generator Guide](https://krishjag.github.io/tealeaf/dotnet/source-generator.html)
