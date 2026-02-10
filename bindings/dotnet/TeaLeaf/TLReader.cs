using System.Dynamic;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using TeaLeaf.Native;

namespace TeaLeaf;

/// <summary>
/// Reader for binary TeaLeaf files (.tlbx).
/// </summary>
public sealed class TLReader : IDisposable
{
    private static readonly JsonSerializerOptions JsonPretty = new()
    {
        WriteIndented = true,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping
    };

    private static readonly JsonSerializerOptions JsonCompact = new()
    {
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping
    };

    private IntPtr _handle;
    private bool _disposed;
    private TLSchema[]? _schemas;
    private Dictionary<string, TLSchema>? _schemaMap;

    private TLReader(IntPtr handle)
    {
        _handle = handle;
    }

    /// <summary>
    /// Open a binary TeaLeaf file for reading.
    /// </summary>
    /// <param name="path">Path to the .tlbx file.</param>
    /// <returns>A reader for the file.</returns>
    /// <exception cref="TLException">Thrown if the file cannot be opened.</exception>
    public static TLReader Open(string path)
    {
        var handle = NativeMethods.tl_reader_open(path);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? $"Failed to open binary TeaLeaf file: {path}";
            throw new TLException(error);
        }
        return new TLReader(handle);
    }

    /// <summary>
    /// Try to open a binary TeaLeaf file.
    /// </summary>
    public static bool TryOpen(string path, out TLReader? reader)
    {
        var handle = NativeMethods.tl_reader_open(path);
        if (handle == IntPtr.Zero)
        {
            reader = null;
            return false;
        }
        reader = new TLReader(handle);
        return true;
    }

    /// <summary>
    /// Open a binary TeaLeaf file with memory mapping (zero-copy access).
    /// This is more efficient for large files as the OS handles paging.
    /// </summary>
    /// <param name="path">Path to the .tlbx file.</param>
    /// <returns>A reader for the file.</returns>
    /// <exception cref="TLException">Thrown if the file cannot be opened.</exception>
    /// <remarks>
    /// The file must not be modified while the reader is open.
    /// </remarks>
    public static TLReader OpenMmap(string path)
    {
        var handle = NativeMethods.tl_reader_open_mmap(path);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? $"Failed to memory-map binary TeaLeaf file: {path}";
            throw new TLException(error);
        }
        return new TLReader(handle);
    }

    /// <summary>
    /// Try to open a binary TeaLeaf file with memory mapping.
    /// </summary>
    public static bool TryOpenMmap(string path, out TLReader? reader)
    {
        var handle = NativeMethods.tl_reader_open_mmap(path);
        if (handle == IntPtr.Zero)
        {
            reader = null;
            return false;
        }
        reader = new TLReader(handle);
        return true;
    }

    /// <summary>
    /// Create a binary TeaLeaf file (.tlbx) from a JSON string.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <param name="outputPath">Path for the output .tlbx file.</param>
    /// <param name="compress">Whether to compress the output.</param>
    /// <exception cref="TLException">Thrown if conversion fails.</exception>
    public static void CreateFromJson(string json, string outputPath, bool compress = true)
    {
        using var doc = TLDocument.FromJson(json);
        doc.Compile(outputPath, compress);
    }

    /// <summary>
    /// Try to create a binary TeaLeaf file from a JSON string.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <param name="outputPath">Path for the output .tlbx file.</param>
    /// <param name="compress">Whether to compress the output.</param>
    /// <returns>True if conversion succeeded.</returns>
    public static bool TryCreateFromJson(string json, string outputPath, bool compress = true)
    {
        try
        {
            CreateFromJson(json, outputPath, compress);
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Get a value from the file by key.
    /// </summary>
    /// <param name="key">The key to look up.</param>
    /// <returns>The value, or null if not found.</returns>
    public TLValue? Get(string key)
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.tl_reader_get(_handle, key);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Get a value as a dynamic object with schema-aware properties.
    /// Arrays of structs become arrays of ExpandoObjects with named fields.
    /// </summary>
    /// <param name="key">The key to look up.</param>
    /// <returns>A dynamic object, or null if not found.</returns>
    public dynamic? GetDynamic(string key)
    {
        ThrowIfDisposed();
        using var value = Get(key);
        if (value == null)
            return null;

        return ConvertToDynamic(value);
    }

    /// <summary>
    /// Get all keys in the file.
    /// </summary>
    public string[] Keys
    {
        get
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.tl_reader_keys(_handle);
            return NativeMethods.PtrToStringArrayAndFree(ptr);
        }
    }

    /// <summary>
    /// Get all schemas defined in the file.
    /// </summary>
    public IReadOnlyList<TLSchema> Schemas
    {
        get
        {
            ThrowIfDisposed();
            LoadSchemas();
            return _schemas!;
        }
    }

    /// <summary>
    /// Get a schema by name.
    /// </summary>
    public TLSchema? GetSchema(string name)
    {
        ThrowIfDisposed();
        LoadSchemas();
        return _schemaMap!.TryGetValue(name, out var schema) ? schema : null;
    }

    /// <summary>
    /// Indexer for key-based access.
    /// </summary>
    public TLValue? this[string key] => Get(key);

    /// <summary>
    /// Check if the file contains a key.
    /// </summary>
    public bool ContainsKey(string key)
    {
        return Get(key) != null;
    }

    /// <summary>
    /// Convert the binary file to pretty-printed JSON.
    /// </summary>
    /// <returns>A JSON string representation of the file contents.</returns>
    public string ToJson()
    {
        ThrowIfDisposed();
        var jsonObj = BuildJsonObject();
        return JsonSerializer.Serialize(jsonObj, JsonPretty);
    }

    /// <summary>
    /// Convert the binary file to compact JSON (no extra whitespace).
    /// </summary>
    /// <returns>A compact JSON string representation of the file contents.</returns>
    public string ToJsonCompact()
    {
        ThrowIfDisposed();
        var jsonObj = BuildJsonObject();
        return JsonSerializer.Serialize(jsonObj, JsonCompact);
    }

    /// <summary>
    /// Convert a specific key's value to JSON.
    /// </summary>
    /// <param name="key">The key to convert.</param>
    /// <returns>JSON string, or null if key not found.</returns>
    public string? GetAsJson(string key)
    {
        ThrowIfDisposed();
        using var value = Get(key);
        if (value == null)
            return null;
        var jsonNode = ValueToJsonNode(value);
        return JsonSerializer.Serialize(jsonNode, JsonPretty);
    }

    private JsonObject BuildJsonObject()
    {
        var jsonObj = new JsonObject();
        foreach (var key in Keys)
        {
            using var value = Get(key);
            if (value != null)
            {
                jsonObj[key] = ValueToJsonNode(value);
            }
        }
        return jsonObj;
    }

    private JsonNode? ValueToJsonNode(TLValue value)
    {
        return value.Type switch
        {
            TLType.Null => null,
            TLType.Bool => JsonValue.Create(value.AsBool()),
            TLType.Int => JsonValue.Create(value.AsInt()),
            TLType.UInt => JsonValue.Create(value.AsUInt()),
            TLType.Float => JsonValue.Create(value.AsFloat()),
            TLType.String => JsonValue.Create(value.AsString()),
            TLType.Bytes => JsonValue.Create(BytesToHexString(value)),
            TLType.Timestamp => JsonValue.Create(TimestampToIso8601(value.AsTimestamp())),
            TLType.Array => ArrayToJsonArray(value),
            TLType.Object => ObjectToJsonObject(value),
            TLType.Map => MapToJsonArray(value),
            TLType.Ref => RefToJsonObject(value),
            TLType.Tagged => TaggedToJsonObject(value),
            _ => null
        };
    }

    private static string? BytesToHexString(TLValue value)
    {
        var bytes = value.AsBytes();
        if (bytes == null || bytes.Length == 0) return "0x";
        return "0x" + BitConverter.ToString(bytes).Replace("-", "").ToLowerInvariant();
    }

    private static string? TimestampToIso8601(long? timestamp)
    {
        if (timestamp == null) return null;
        var dt = DateTimeOffset.FromUnixTimeMilliseconds(timestamp.Value);
        return dt.ToString("yyyy-MM-ddTHH:mm:ss.fffZ");
    }

    private JsonArray ArrayToJsonArray(TLValue value)
    {
        var arr = new JsonArray();
        var length = value.ArrayLength;
        for (int i = 0; i < length; i++)
        {
            using var element = value.GetArrayElement(i);
            arr.Add(element != null ? ValueToJsonNode(element) : null);
        }
        return arr;
    }

    private JsonObject ObjectToJsonObject(TLValue value)
    {
        var obj = new JsonObject();
        var keys = value.GetObjectKeys();
        foreach (var key in keys)
        {
            using var fieldValue = value.GetField(key);
            obj[key] = fieldValue != null ? ValueToJsonNode(fieldValue) : null;
        }
        return obj;
    }

    private JsonArray MapToJsonArray(TLValue value)
    {
        // Maps are represented as array of [key, value] pairs
        var arr = new JsonArray();
        var length = value.MapLength;
        for (int i = 0; i < length; i++)
        {
            using var key = value.GetMapKey(i);
            using var mapValue = value.GetMapValue(i);
            if (key != null && mapValue != null)
            {
                var pair = new JsonArray { ValueToJsonNode(key), ValueToJsonNode(mapValue) };
                arr.Add(pair);
            }
        }
        return arr;
    }

    private static JsonObject RefToJsonObject(TLValue value)
    {
        var obj = new JsonObject();
        obj["$ref"] = JsonValue.Create(value.AsRefName());
        return obj;
    }

    private JsonObject TaggedToJsonObject(TLValue value)
    {
        var obj = new JsonObject();
        obj["$tag"] = JsonValue.Create(value.AsTagName());
        using var innerValue = value.AsTagValue();
        obj["$value"] = innerValue != null ? ValueToJsonNode(innerValue) : null;
        return obj;
    }

    private void LoadSchemas()
    {
        if (_schemas != null)
            return;

        var count = (int)NativeMethods.tl_reader_schema_count(_handle);
        _schemas = new TLSchema[count];
        _schemaMap = new Dictionary<string, TLSchema>(count);

        for (int i = 0; i < count; i++)
        {
            var namePtr = NativeMethods.tl_reader_schema_name(_handle, (nuint)i);
            var name = NativeMethods.PtrToStringAndFree(namePtr) ?? $"schema_{i}";

            var fieldCount = (int)NativeMethods.tl_reader_schema_field_count(_handle, (nuint)i);
            var fields = new TLField[fieldCount];

            for (int j = 0; j < fieldCount; j++)
            {
                var fieldNamePtr = NativeMethods.tl_reader_schema_field_name(_handle, (nuint)i, (nuint)j);
                var fieldName = NativeMethods.PtrToStringAndFree(fieldNamePtr) ?? $"field_{j}";

                var fieldTypePtr = NativeMethods.tl_reader_schema_field_type(_handle, (nuint)i, (nuint)j);
                var fieldType = NativeMethods.PtrToStringAndFree(fieldTypePtr) ?? "unknown";

                var nullable = NativeMethods.tl_reader_schema_field_nullable(_handle, (nuint)i, (nuint)j);
                var isArray = NativeMethods.tl_reader_schema_field_is_array(_handle, (nuint)i, (nuint)j);

                fields[j] = new TLField(fieldName, fieldType, nullable, isArray);
            }

            var schema = new TLSchema(name, fields);
            _schemas[i] = schema;
            _schemaMap[name] = schema;
        }
    }

    private dynamic? ConvertToDynamic(TLValue value)
    {
        return value.Type switch
        {
            TLType.Null => null,
            TLType.Bool => value.AsBool(),
            TLType.Int => value.AsInt(),
            TLType.UInt => value.AsUInt(),
            TLType.Float => value.AsFloat(),
            TLType.String => value.AsString(),
            TLType.Bytes => value.AsBytes(),
            TLType.Timestamp => value.AsDateTime(),
            TLType.Array => ConvertArrayToDynamic(value),
            TLType.Object => ConvertObjectToDynamic(value),
            TLType.Map => ConvertMapToDynamic(value),
            TLType.Ref => value.AsRefName(),
            TLType.Tagged => ConvertTaggedToDynamic(value),
            _ => null
        };
    }

    private dynamic?[] ConvertArrayToDynamic(TLValue value)
    {
        var length = value.ArrayLength;
        var result = new dynamic?[length];

        for (int i = 0; i < length; i++)
        {
            using var element = value.GetArrayElement(i);
            result[i] = element != null ? ConvertToDynamic(element) : null;
        }

        return result;
    }

    private ExpandoObject ConvertObjectToDynamic(TLValue value)
    {
        var expando = new ExpandoObject();
        var dict = (IDictionary<string, object?>)expando;

        var keys = value.GetObjectKeys();
        foreach (var key in keys)
        {
            using var fieldValue = value.GetField(key);
            dict[key] = fieldValue != null ? ConvertToDynamic(fieldValue) : null;
        }

        return expando;
    }

    private KeyValuePair<dynamic?, dynamic?>[] ConvertMapToDynamic(TLValue value)
    {
        var length = value.MapLength;
        var result = new KeyValuePair<dynamic?, dynamic?>[length];

        for (int i = 0; i < length; i++)
        {
            using var key = value.GetMapKey(i);
            using var mapValue = value.GetMapValue(i);
            result[i] = new KeyValuePair<dynamic?, dynamic?>(
                key != null ? ConvertToDynamic(key) : null,
                mapValue != null ? ConvertToDynamic(mapValue) : null
            );
        }

        return result;
    }

    private ExpandoObject ConvertTaggedToDynamic(TLValue value)
    {
        var expando = new ExpandoObject();
        var dict = (IDictionary<string, object?>)expando;
        dict["$tag"] = value.AsTagName();
        using var innerValue = value.AsTagValue();
        dict["$value"] = innerValue != null ? ConvertToDynamic(innerValue) : null;
        return expando;
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(TLReader));
    }

    public void Dispose()
    {
        if (!_disposed && _handle != IntPtr.Zero)
        {
            NativeMethods.tl_reader_free(_handle);
            _handle = IntPtr.Zero;
            _disposed = true;
        }
    }
}
