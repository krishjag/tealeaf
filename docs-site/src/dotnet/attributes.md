# .NET Attributes Reference

All TeaLeaf annotations are in the `TeaLeaf.Annotations` namespace.

## Type-Level Attributes

### `[TeaLeaf]`

Marks a class for TeaLeaf serialization. Used by both the reflection-based `TeaLeafSerializer` (runtime) and the source generator (compile-time).

```csharp
[TeaLeaf]                                 // Reflection-only, no partial needed
public class MyClass { }

[TeaLeaf(StructName = "config")]          // Override schema name (default: snake_case of class name)
public class AppConfiguration { }
```

#### `Generate` property

Set `Generate = true` to enable compile-time source generation. Requires the class to be declared as `partial`:

```csharp
[TeaLeaf(Generate = true)]               // Source generation enabled
public partial class MyClass { }
```

#### `EmitSchema` property

Defaults to `true`. When set to `false`, the source generator skips `@struct` and `@table` output for arrays of this type:

```csharp
[TeaLeaf(Generate = true, EmitSchema = false)]  // Data only, no @struct definition
public partial class RawData { }
```

### `[TLKey("key_name")]`

Overrides the top-level key used when serializing as a document entry:

```csharp
[TeaLeaf(Generate = true)]
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
public class User
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
public class User
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
public class User
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
public class Event
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
| `[TeaLeaf]` | Class | Mark for TeaLeaf serialization, optional struct name via `StructName` |
| `[TeaLeaf(Generate = true)]` | Class | Enable compile-time source generation (requires `partial`) |
| `[TLKey("key")]` | Class | Override document key |
| `[TLSkip]` | Property | Exclude from serialization |
| `[TLOptional]` | Property | Mark as nullable in schema |
| `[TLRename("name")]` | Property | Override field name |
| `[TLType("type")]` | Property | Override TeaLeaf type |

## Combining Attributes

```csharp
[TeaLeaf(Generate = true, StructName = "event_record")]
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
