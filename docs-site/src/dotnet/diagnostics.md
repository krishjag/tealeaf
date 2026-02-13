# Diagnostics

The TeaLeaf source generator reports diagnostics (warnings and errors) through the standard C# compiler diagnostic system.

## Diagnostic Codes

| Code | Severity | Message |
|------|----------|---------|
| **TL001** | Error | Type must be declared as partial |
| **TL002** | Warning | Unsupported property type |
| **TL003** | Error | Invalid TLType attribute value |
| **TL004** | Warning | Nested type not annotated with [TeaLeaf] |
| **TL005** | Warning | Circular type reference detected |
| **TL006** | Error | Open generic types are not supported |

## TL001: Type Must Be Partial

When `Generate = true` is set, the source generator needs to add methods to your class. This requires the `partial` modifier.

```csharp
// ERROR: TL001
[TeaLeaf(Generate = true)]
public class User { }  // Missing 'partial'

// FIXED
[TeaLeaf(Generate = true)]
public partial class User { }
```

> **Note:** `[TeaLeaf]` without `Generate = true` does **not** trigger TL001 and does not require `partial`. It is used for reflection-based serialization only.

## TL002: Unsupported Property Type

A property type isn't directly mappable to a TeaLeaf type.

```csharp
[TeaLeaf(Generate = true)]
public partial class Config
{
    public IntPtr NativeHandle { get; set; }  // WARNING: TL002
}
```

The property will be skipped. Supported types include all primitives, `string`, `DateTime`, `DateTimeOffset`, `byte[]`, `List<T>`, `Dictionary<string, T>`, enums, and other `[TeaLeaf]`-annotated classes.

## TL003: Invalid TLType Value

The `[TLType]` attribute was given an unrecognized type name.

```csharp
[TeaLeaf(Generate = true)]
public partial class Event
{
    [TLType("datetime")]   // ERROR: TL003 -- "datetime" is not a valid type
    public long Created { get; set; }

    [TLType("timestamp")]  // CORRECT
    public long Updated { get; set; }
}
```

Valid values: `bool`, `int`, `int8`, `int16`, `int32`, `int64`, `uint`, `uint8`, `uint16`, `uint32`, `uint64`, `float`, `float32`, `float64`, `string`, `bytes`, `timestamp`.

## TL004: Nested Type Not Annotated

A property references a class type that doesn't have the `[TeaLeaf]` attribute.

```csharp
public class Address  // Missing [TeaLeaf]
{
    public string City { get; set; } = "";
}

[TeaLeaf(Generate = true)]
public partial class User
{
    public Address Home { get; set; } = new();  // WARNING: TL004
}
```

Fix by adding `[TeaLeaf]` to the nested type:

```csharp
[TeaLeaf(Generate = true)]
public partial class Address
{
    public string City { get; set; } = "";
}
```

## TL005: Circular Type Reference

A type references itself (directly or transitively), which may cause a stack overflow at runtime during serialization.

```csharp
[TeaLeaf(Generate = true)]
public partial class TreeNode
{
    public string Name { get; set; } = "";
    public TreeNode? Child { get; set; }  // WARNING: TL005 -- circular reference
}
```

The code will still compile, but recursive structures must be bounded (e.g., use `[TLOptional]` with null termination) to avoid infinite recursion.

## TL006: Open Generic Types

Generic type parameters are not supported:

```csharp
// ERROR: TL006
[TeaLeaf(Generate = true)]
public partial class Container<T>
{
    public T Value { get; set; }
}
```

Use concrete types instead. For generic scenarios, use the [Reflection Serializer](./serializer.md).

## Viewing Diagnostics

Diagnostics appear in:
- **Visual Studio** -- Error List window
- **VS Code** -- Problems panel (with C# extension)
- **dotnet build** -- terminal output
- **MSBuild** -- build log

Example compiler output:
```
User.cs(3,22): error TL001: TeaLeaf type 'User' must be declared as partial
Config.cs(8,16): warning TL004: Property 'Address' type is not annotated with [TeaLeaf]
```
