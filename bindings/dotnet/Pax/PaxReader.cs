using System.Dynamic;
using System.Text.Json;
using System.Text.Json.Nodes;
using Pax.Native;

namespace Pax;

/// <summary>
/// Reader for binary Pax files (.paxb).
/// </summary>
public sealed class PaxReader : IDisposable
{
    private IntPtr _handle;
    private bool _disposed;
    private PaxSchema[]? _schemas;
    private Dictionary<string, PaxSchema>? _schemaMap;

    private PaxReader(IntPtr handle)
    {
        _handle = handle;
    }

    /// <summary>
    /// Open a binary Pax file for reading.
    /// </summary>
    /// <param name="path">Path to the .paxb file.</param>
    /// <returns>A reader for the file.</returns>
    /// <exception cref="PaxException">Thrown if the file cannot be opened.</exception>
    public static PaxReader Open(string path)
    {
        var handle = NativeMethods.pax_reader_open(path);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? $"Failed to open binary Pax file: {path}";
            throw new PaxException(error);
        }
        return new PaxReader(handle);
    }

    /// <summary>
    /// Try to open a binary Pax file.
    /// </summary>
    public static bool TryOpen(string path, out PaxReader? reader)
    {
        var handle = NativeMethods.pax_reader_open(path);
        if (handle == IntPtr.Zero)
        {
            reader = null;
            return false;
        }
        reader = new PaxReader(handle);
        return true;
    }

    /// <summary>
    /// Open a binary Pax file with memory mapping (zero-copy access).
    /// This is more efficient for large files as the OS handles paging.
    /// </summary>
    /// <param name="path">Path to the .paxb file.</param>
    /// <returns>A reader for the file.</returns>
    /// <exception cref="PaxException">Thrown if the file cannot be opened.</exception>
    /// <remarks>
    /// The file must not be modified while the reader is open.
    /// </remarks>
    public static PaxReader OpenMmap(string path)
    {
        var handle = NativeMethods.pax_reader_open_mmap(path);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? $"Failed to memory-map binary Pax file: {path}";
            throw new PaxException(error);
        }
        return new PaxReader(handle);
    }

    /// <summary>
    /// Try to open a binary Pax file with memory mapping.
    /// </summary>
    public static bool TryOpenMmap(string path, out PaxReader? reader)
    {
        var handle = NativeMethods.pax_reader_open_mmap(path);
        if (handle == IntPtr.Zero)
        {
            reader = null;
            return false;
        }
        reader = new PaxReader(handle);
        return true;
    }

    /// <summary>
    /// Create a binary Pax file (.paxb) from a JSON string.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <param name="outputPath">Path for the output .paxb file.</param>
    /// <param name="compress">Whether to compress the output.</param>
    /// <exception cref="PaxException">Thrown if conversion fails.</exception>
    public static void CreateFromJson(string json, string outputPath, bool compress = true)
    {
        using var doc = PaxDocument.FromJson(json);
        doc.Compile(outputPath, compress);
    }

    /// <summary>
    /// Try to create a binary Pax file from a JSON string.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <param name="outputPath">Path for the output .paxb file.</param>
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
    public PaxValue? Get(string key)
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.pax_reader_get(_handle, key);
        return ptr == IntPtr.Zero ? null : new PaxValue(ptr);
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
            var ptr = NativeMethods.pax_reader_keys(_handle);
            return NativeMethods.PtrToStringArrayAndFree(ptr);
        }
    }

    /// <summary>
    /// Get all schemas defined in the file.
    /// </summary>
    public IReadOnlyList<PaxSchema> Schemas
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
    public PaxSchema? GetSchema(string name)
    {
        ThrowIfDisposed();
        LoadSchemas();
        return _schemaMap!.TryGetValue(name, out var schema) ? schema : null;
    }

    /// <summary>
    /// Indexer for key-based access.
    /// </summary>
    public PaxValue? this[string key] => Get(key);

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
        return JsonSerializer.Serialize(jsonObj, new JsonSerializerOptions { WriteIndented = true });
    }

    /// <summary>
    /// Convert the binary file to compact JSON (no extra whitespace).
    /// </summary>
    /// <returns>A compact JSON string representation of the file contents.</returns>
    public string ToJsonCompact()
    {
        ThrowIfDisposed();
        var jsonObj = BuildJsonObject();
        return JsonSerializer.Serialize(jsonObj);
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
        return JsonSerializer.Serialize(jsonNode, new JsonSerializerOptions { WriteIndented = true });
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

    private JsonNode? ValueToJsonNode(PaxValue value)
    {
        return value.Type switch
        {
            PaxType.Null => null,
            PaxType.Bool => JsonValue.Create(value.AsBool()),
            PaxType.Int => JsonValue.Create(value.AsInt()),
            PaxType.UInt => JsonValue.Create(value.AsUInt()),
            PaxType.Float => JsonValue.Create(value.AsFloat()),
            PaxType.String => JsonValue.Create(value.AsString()),
            PaxType.Bytes => JsonValue.Create(BytesToHexString(value)),
            PaxType.Timestamp => JsonValue.Create(TimestampToIso8601(value.AsTimestamp())),
            PaxType.Array => ArrayToJsonArray(value),
            PaxType.Object => ObjectToJsonObject(value),
            PaxType.Map => MapToJsonArray(value),
            PaxType.Ref => RefToJsonObject(value),
            PaxType.Tagged => TaggedToJsonObject(value),
            _ => null
        };
    }

    private static string? BytesToHexString(PaxValue value)
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

    private JsonArray ArrayToJsonArray(PaxValue value)
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

    private JsonObject ObjectToJsonObject(PaxValue value)
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

    private JsonArray MapToJsonArray(PaxValue value)
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

    private static JsonObject RefToJsonObject(PaxValue value)
    {
        var obj = new JsonObject();
        obj["$ref"] = JsonValue.Create(value.AsRefName());
        return obj;
    }

    private JsonObject TaggedToJsonObject(PaxValue value)
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

        var count = (int)NativeMethods.pax_reader_schema_count(_handle);
        _schemas = new PaxSchema[count];
        _schemaMap = new Dictionary<string, PaxSchema>(count);

        for (int i = 0; i < count; i++)
        {
            var namePtr = NativeMethods.pax_reader_schema_name(_handle, (nuint)i);
            var name = NativeMethods.PtrToStringAndFree(namePtr) ?? $"schema_{i}";

            var fieldCount = (int)NativeMethods.pax_reader_schema_field_count(_handle, (nuint)i);
            var fields = new PaxField[fieldCount];

            for (int j = 0; j < fieldCount; j++)
            {
                var fieldNamePtr = NativeMethods.pax_reader_schema_field_name(_handle, (nuint)i, (nuint)j);
                var fieldName = NativeMethods.PtrToStringAndFree(fieldNamePtr) ?? $"field_{j}";

                var fieldTypePtr = NativeMethods.pax_reader_schema_field_type(_handle, (nuint)i, (nuint)j);
                var fieldType = NativeMethods.PtrToStringAndFree(fieldTypePtr) ?? "unknown";

                var nullable = NativeMethods.pax_reader_schema_field_nullable(_handle, (nuint)i, (nuint)j);
                var isArray = NativeMethods.pax_reader_schema_field_is_array(_handle, (nuint)i, (nuint)j);

                fields[j] = new PaxField(fieldName, fieldType, nullable, isArray);
            }

            var schema = new PaxSchema(name, fields);
            _schemas[i] = schema;
            _schemaMap[name] = schema;
        }
    }

    private dynamic? ConvertToDynamic(PaxValue value)
    {
        return value.Type switch
        {
            PaxType.Null => null,
            PaxType.Bool => value.AsBool(),
            PaxType.Int => value.AsInt(),
            PaxType.UInt => value.AsUInt(),
            PaxType.Float => value.AsFloat(),
            PaxType.String => value.AsString(),
            PaxType.Bytes => value.AsBytes(),
            PaxType.Timestamp => value.AsDateTime(),
            PaxType.Array => ConvertArrayToDynamic(value),
            PaxType.Object => ConvertObjectToDynamic(value),
            PaxType.Map => ConvertMapToDynamic(value),
            PaxType.Ref => value.AsRefName(),
            PaxType.Tagged => ConvertTaggedToDynamic(value),
            _ => null
        };
    }

    private dynamic?[] ConvertArrayToDynamic(PaxValue value)
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

    private ExpandoObject ConvertObjectToDynamic(PaxValue value)
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

    private KeyValuePair<dynamic?, dynamic?>[] ConvertMapToDynamic(PaxValue value)
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

    private ExpandoObject ConvertTaggedToDynamic(PaxValue value)
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
            throw new ObjectDisposedException(nameof(PaxReader));
    }

    public void Dispose()
    {
        if (!_disposed && _handle != IntPtr.Zero)
        {
            NativeMethods.pax_reader_free(_handle);
            _handle = IntPtr.Zero;
            _disposed = true;
        }
    }
}
