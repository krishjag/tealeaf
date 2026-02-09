# .NET Attributes Reference

All TeaLeaf annotations are in the `TeaLeaf.Annotations` namespace.

## Type-Level Attributes

### `[TeaLeaf]` / `[TeaLeaf("struct_name")]`

Marks a class for source generator processing:

```csharp
[TeaLeaf]           // Schema name: "my_class" (auto snake_case)
public partial class MyClass { }

[TeaLeaf("config")] // Schema name: "config" (explicit)
public partial class AppConfiguration { }
```

The optional string parameter sets the struct name used in the TeaLeaf schema. If omitted, the class name is converted to snake_case.

The attribute also has an `EmitSchema` property (defaults to `true`). When set to `false`, the source generator skips `@struct` and `@table` output for arrays of this type:

```csharp
[TeaLeaf(EmitSchema = false)]  // Data only, no @struct definition
public partial class RawData { }
```

### `[TLKey("key_name")]`

Overrides the top-level key used when serializing as a document entry:

```csharp
[TeaLeaf]
[TLKey("app_settings")]
public partial class Config
{
    public string Host { get; set; } = "";
    public int Port { get; set; }
}

// Default key would be "config", but TLKey overrides to "app_settings"
string doc = config.ToTeaLeafDocument(); // key is "app_settings"
```

## Property-Level Attributes

### `[TLSkip]`

Exclude a property from serialization and deserialization:

```csharp
[TeaLeaf]
public partial class User
{
    public int Id { get; set; }
    public string Name { get; set; } = "";

    [TLSkip]
    public string ComputedDisplayName => $"User #{Id}: {Name}";
}
```

### `[TLOptional]`

Mark a property as nullable in the schema:

```csharp
[TeaLeaf]
public partial class User
{
    public string Name { get; set; } = "";

    [TLOptional]
    public string? Email { get; set; }

    [TLOptional]
    public int? Age { get; set; }
}
// Schema: @struct user (name: string, email: string?, age: int?)
```

> **Note:** Properties of nullable reference types (`string?`) or `Nullable<T>` types (`int?`) are automatically treated as optional. The `[TLOptional]` attribute is mainly for explicit documentation.

### `[TLRename("field_name")]`

Override the field name in the TeaLeaf schema:

```csharp
[TeaLeaf]
public partial class User
{
    [TLRename("user_name")]
    public string Name { get; set; } = "";

    [TLRename("is_active")]
    public bool Active { get; set; }
}
// Schema: @struct user (user_name: string, is_active: bool)
```

Without `[TLRename]`, property names are converted to snake_case (`Name` → `name`, `IsActive` → `is_active`).

### `[TLType("type_name")]`

Override the TeaLeaf type for a field:

```csharp
[TeaLeaf]
public partial class Event
{
    public string Name { get; set; } = "";

    [TLType("timestamp")]
    public long CreatedAt { get; set; }  // Would be int64, forced to timestamp

    [TLType("uint64")]
    public long LargeCount { get; set; }  // Would be int64, forced to uint64
}
```

Valid type names: `bool`, `int`, `int8`, `int16`, `int32`, `int64`, `uint`, `uint8`, `uint16`, `uint32`, `uint64`, `float`, `float32`, `float64`, `string`, `bytes`, `timestamp`.

## Attribute Summary

| Attribute | Level | Description |
|-----------|-------|-------------|
| `[TeaLeaf]` / `[TeaLeaf("name")]` | Class | Enable source generation, optional struct name |
| `[TLKey("key")]` | Class | Override document key |
| `[TLSkip]` | Property | Exclude from serialization |
| `[TLOptional]` | Property | Mark as nullable in schema |
| `[TLRename("name")]` | Property | Override field name |
| `[TLType("type")]` | Property | Override TeaLeaf type |

## Combining Attributes

```csharp
[TeaLeaf("event_record")]
[TLKey("events")]
public partial class EventRecord
{
    [TLRename("event_id")]
    public int Id { get; set; }

    public string Name { get; set; } = "";

    [TLType("timestamp")]
    public long CreatedAt { get; set; }

    [TLOptional]
    [TLRename("extra_data")]
    public string? Metadata { get; set; }

    [TLSkip]
    public string DisplayLabel => $"{Name} ({Id})";
}
```

Generated schema:
```tl
@struct event_record (event_id: int, name: string, created_at: timestamp, extra_data: string?)
```
