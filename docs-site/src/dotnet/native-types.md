# Native Types

The managed wrapper types provide safe access to the native TeaLeaf library. All native types implement `IDisposable` and must be disposed to prevent memory leaks.

## TLDocument

Represents a parsed TeaLeaf document.

### Construction

```csharp
// Parse text
using var doc = TLDocument.Parse("name: alice\nage: 30");

// Parse from file (text or binary -- auto-detected)
using var doc = TLDocument.ParseFile("data.tl");
using var doc = TLDocument.ParseFile("data.tlbx");

// From JSON string
using var doc = TLDocument.FromJson("{\"name\": \"alice\"}");
```

### Value Access

```csharp
// Get value by key
using var val = doc["name"];       // indexer
using var val = doc.Get("name");   // method

// Get all keys
string[] keys = doc.Keys;
```

### Output

```csharp
// To text
string text = doc.ToText();               // full document (schemas + data)
string data = doc.ToTextDataOnly();       // data only (no schemas)

// To JSON
string json = doc.ToJson();               // pretty-printed
string json = doc.ToJsonCompact();        // minified

// Compile to binary
doc.Compile("output.tlbx", compress: true);
```

### Disposal

`TLDocument` wraps a native pointer. Always dispose:

```csharp
using var doc = TLDocument.Parse(text);  // using statement (recommended)

// Or manual disposal
var doc = TLDocument.Parse(text);
try { /* use doc */ }
finally { doc.Dispose(); }
```

## TLValue

Represents any TeaLeaf value with type-safe accessors.

### Type Checking

```csharp
TLType type = value.Type;    // Enum: Null, Bool, Int, UInt, Float, String, etc.
bool isNull = value.IsNull;  // Shorthand for Type == TLType.Null
```

### Primitive Accessors

Each returns `null` if the value is not the expected type:

```csharp
bool? b = value.AsBool();
long? i = value.AsInt();
ulong? u = value.AsUInt();
double? f = value.AsFloat();
string? s = value.AsString();
long? ts = value.AsTimestamp();          // Unix milliseconds
DateTimeOffset? dt = value.AsDateTime(); // Converted from timestamp
byte[]? bytes = value.AsBytes();
```

### Object Access

```csharp
string[] keys = value.ObjectKeys;          // All field names
using var field = value.GetField("name");  // Get by key
using var field = value["name"];           // Indexer shorthand
```

### Array Access

```csharp
int len = value.ArrayLength;
using var elem = value.GetArrayElement(0); // By index
using var elem = value[0];                 // Indexer shorthand

foreach (var item in value.AsArray())
{
    // item is a TLValue -- caller must dispose
    using (item)
    {
        Console.WriteLine(item.AsString());
    }
}
```

### Map Access

```csharp
int len = value.MapLength;
using var key = value.GetMapKey(0);
using var val = value.GetMapValue(0);

foreach (var (k, v) in value.AsMap())
{
    using (k) using (v)
    {
        Console.WriteLine($"{k.AsString()}: {v.AsString()}");
    }
}
```

### Reference and Tag Access

```csharp
string? refName = value.AsRefName();   // For Ref values
string? tagName = value.AsTagName();   // For Tagged values
using var inner = value.AsTagValue();  // Inner value of a Tagged
```

### Dynamic Conversion

```csharp
object? obj = value.ToObject();
// Returns: bool, long, ulong, double, string, byte[],
// DateTimeOffset, object[], Dictionary<string, object?>, or null
```

## TLReader

Binary file reader with optional memory-mapped I/O.

### Construction

```csharp
// Standard file read
using var reader = TLReader.Open("data.tlbx");

// Memory-mapped (recommended for large files)
using var reader = TLReader.OpenMmap("data.tlbx");
```

### Value Access

```csharp
string[] keys = reader.Keys;
using var val = reader["users"];
using var val = reader.Get("users");
```

### Schema Introspection

```csharp
int schemaCount = reader.SchemaCount;

for (int i = 0; i < schemaCount; i++)
{
    string name = reader.GetSchemaName(i);
    int fieldCount = reader.GetSchemaFieldCount(i);

    Console.WriteLine($"Schema: {name}");
    for (int j = 0; j < fieldCount; j++)
    {
        string fname = reader.GetSchemaFieldName(i, j);
        string ftype = reader.GetSchemaFieldType(i, j);
        bool nullable = reader.GetSchemaFieldNullable(i, j);
        bool isArray = reader.GetSchemaFieldIsArray(i, j);

        Console.WriteLine($"  {fname}: {(isArray ? "[]" : "")}{ftype}{(nullable ? "?" : "")}");
    }
}
```

## TLType Enum

```csharp
public enum TLType
{
    Null = 0,
    Bool = 1,
    Int = 2,
    UInt = 3,
    Float = 4,
    String = 5,
    Bytes = 6,
    Array = 7,
    Object = 8,
    Map = 9,
    Ref = 10,
    Tagged = 11,
    Timestamp = 12,
}
```

## Memory Management

All native types (`TLDocument`, `TLValue`, `TLReader`) hold native pointers and **must be disposed**:

```csharp
// Preferred: using statement
using var doc = TLDocument.Parse(text);

// For values from collections, dispose each item:
foreach (var item in value.AsArray())
{
    using (item)
    {
        // process
    }
}

// For map entries:
foreach (var (key, val) in value.AsMap())
{
    using (key) using (val)
    {
        // process
    }
}
```

Accessing a disposed object throws `ObjectDisposedException`.
