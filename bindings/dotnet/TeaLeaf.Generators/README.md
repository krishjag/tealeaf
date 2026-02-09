# TeaLeaf.Generators

C# incremental source generator for the TeaLeaf data format. Generates compile-time serialization and deserialization code for DTOs annotated with `[TeaLeaf]`.

## Installation

```bash
dotnet add package TeaLeaf.Generators
```

> **Note**: You also need the [TeaLeaf](https://www.nuget.org/packages/TeaLeaf) package for runtime APIs (`TLDocument`, `TLReader`, etc.).

## Usage

Annotate your classes with `[TeaLeaf]` from `TeaLeaf.Annotations`:

```csharp
using TeaLeaf.Annotations;

[TeaLeaf]
public partial class Employee
{
    public string Name { get; set; } = "";
    public int Age { get; set; }

    [TLRename("dept")]
    public string Department { get; set; } = "";
}
```

The source generator produces methods on the partial class at compile time:

- `ToTeaLeafText()` -- serialize to TeaLeaf text with schemas
- `ToTeaLeafDocument(key?)` -- create a full `TLDocument` with schemas
- `CompileToTeaLeaf(path, key?, compress?)` -- compile directly to binary `.tlbx`
- `FromTeaLeaf(TLDocument doc, key?)` -- deserialize from a TeaLeaf document (static)
- `FromTeaLeaf(TLValue value)` -- deserialize from a TeaLeaf value (static)
- `GetTeaLeafSchema()` -- get the `@struct` schema definition (static)

## Why Use the Source Generator?

- **No reflection**: All serialization code is generated at compile time
- **AOT-compatible**: Works with Native AOT and trimming
- **Build-time validation**: Diagnostics (TL001-TL006) catch issues during compilation
- **Better performance**: No runtime type inspection overhead

## Diagnostics

| Code | Description |
|------|-------------|
| TL001 | Type must be a class or record |
| TL002 | Type must be partial |
| TL003 | Unsupported property type |
| TL004 | Duplicate field name |
| TL005 | Missing parameterless constructor |
| TL006 | Property must have getter and setter |

## License

MIT

## Links

- [GitHub Repository](https://github.com/krishjag/tealeaf)
- [Source Generator Guide](https://krishjag.github.io/tealeaf/dotnet/source-generator.html)
- [.NET Guide](https://krishjag.github.io/tealeaf/dotnet/overview.html)
