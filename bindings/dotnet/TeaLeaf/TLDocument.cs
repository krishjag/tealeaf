using TeaLeaf.Native;

namespace TeaLeaf;

/// <summary>
/// Represents a parsed TeaLeaf document.
/// </summary>
public sealed class TLDocument : IDisposable
{
    private IntPtr _handle;
    private bool _disposed;
    private TLSchema[]? _schemas;
    private Dictionary<string, TLSchema>? _schemaMap;

    private TLDocument(IntPtr handle)
    {
        _handle = handle;
    }

    /// <summary>
    /// Parse a TeaLeaf document from a text string.
    /// </summary>
    /// <param name="text">The TeaLeaf text to parse.</param>
    /// <returns>A parsed TLDocument.</returns>
    /// <exception cref="TLException">Thrown if parsing fails.</exception>
    public static TLDocument Parse(string text)
    {
        var handle = NativeMethods.tl_parse(text);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? "Failed to parse TeaLeaf document";
            throw new TLException(error);
        }
        return new TLDocument(handle);
    }

    /// <summary>
    /// Parse a TeaLeaf document from a file.
    /// </summary>
    /// <param name="path">Path to the .tl file.</param>
    /// <returns>A parsed TLDocument.</returns>
    /// <exception cref="TLException">Thrown if parsing fails.</exception>
    public static TLDocument ParseFile(string path)
    {
        var handle = NativeMethods.tl_parse_file(path);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? $"Failed to parse TeaLeaf file: {path}";
            throw new TLException(error);
        }
        return new TLDocument(handle);
    }

    /// <summary>
    /// Try to parse a TeaLeaf document from text.
    /// </summary>
    /// <param name="text">The TeaLeaf text to parse.</param>
    /// <param name="document">The parsed document, if successful.</param>
    /// <returns>True if parsing succeeded.</returns>
    public static bool TryParse(string text, out TLDocument? document)
    {
        var handle = NativeMethods.tl_parse(text);
        if (handle == IntPtr.Zero)
        {
            document = null;
            return false;
        }
        document = new TLDocument(handle);
        return true;
    }

    /// <summary>
    /// Create a TeaLeaf document from a JSON string with automatic schema inference.
    /// Uniform object arrays are converted to @struct definitions and @table format.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <returns>A TLDocument containing the JSON data with inferred schemas.</returns>
    /// <exception cref="TLException">Thrown if JSON parsing fails.</exception>
    public static TLDocument FromJson(string json)
    {
        var handle = NativeMethods.tl_document_from_json(json);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? "Failed to parse JSON";
            throw new TLException(error);
        }
        return new TLDocument(handle);
    }

    /// <summary>
    /// Try to create a TeaLeaf document from a JSON string with schema inference.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <param name="document">The parsed document, if successful.</param>
    /// <returns>True if parsing succeeded.</returns>
    public static bool TryFromJson(string json, out TLDocument? document)
    {
        var handle = NativeMethods.tl_document_from_json(json);
        if (handle == IntPtr.Zero)
        {
            document = null;
            return false;
        }
        document = new TLDocument(handle);
        return true;
    }

    /// <summary>
    /// Get a value from the document by key.
    /// </summary>
    /// <param name="key">The key to look up.</param>
    /// <returns>The value, or null if not found.</returns>
    public TLValue? Get(string key)
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.tl_document_get(_handle, key);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Get all keys in the document.
    /// </summary>
    public string[] Keys
    {
        get
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.tl_document_keys(_handle);
            return NativeMethods.PtrToStringArrayAndFree(ptr);
        }
    }

    /// <summary>
    /// Indexer for key-based access.
    /// </summary>
    public TLValue? this[string key] => Get(key);

    /// <summary>
    /// Navigate a dot-path expression to reach a deeply nested value.
    /// The first segment is used as the document key; remaining segments
    /// traverse nested objects and arrays.
    /// Implemented as a single native call for efficiency.
    /// </summary>
    /// <param name="path">
    /// A dot-separated path such as <c>"order.items[0].product.price.base_price"</c>.
    /// The first segment (<c>order</c>) is the top-level document key.
    /// </param>
    /// <returns>The value at the path, or null if any segment is missing. The caller must dispose.</returns>
    public TLValue? GetPath(string path)
    {
        ThrowIfDisposed();
        if (string.IsNullOrEmpty(path)) return null;
        var ptr = NativeMethods.tl_document_get_path(_handle, path);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Convert the document to TeaLeaf text format.
    /// </summary>
    /// <param name="compact">If true, removes insignificant whitespace for token-efficient output.</param>
    /// <param name="compactFloats">If true, strips .0 from whole-number floats (e.g., 42.0 becomes 42).
    /// Note: re-parsing will produce Int instead of Float for these values.</param>
    /// <param name="ignoreSchemas">If true, omits @struct definitions and outputs data only.</param>
    public string ToText(bool compact = false, bool compactFloats = false, bool ignoreSchemas = false)
    {
        ThrowIfDisposed();
        var ptr = ignoreSchemas
            ? NativeMethods.tl_document_to_text_data_only_with_options(_handle, compact, compactFloats)
            : NativeMethods.tl_document_to_text_with_options(_handle, compact, compactFloats);
        return NativeMethods.PtrToStringAndFree(ptr) ?? string.Empty;
    }

    /// <summary>
    /// Convert the document to pretty-printed JSON.
    /// </summary>
    /// <returns>A JSON string representation of the document.</returns>
    public string ToJson()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.tl_document_to_json(_handle);
        return NativeMethods.PtrToStringAndFree(ptr) ?? "{}";
    }

    /// <summary>
    /// Convert the document to compact JSON (no extra whitespace).
    /// </summary>
    /// <returns>A compact JSON string representation of the document.</returns>
    public string ToJsonCompact()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.tl_document_to_json_compact(_handle);
        return NativeMethods.PtrToStringAndFree(ptr) ?? "{}";
    }

    /// <summary>
    /// Compile the document to binary format and write to a file.
    /// </summary>
    /// <param name="path">Output file path (.tlbx).</param>
    /// <param name="compress">Whether to compress the output.</param>
    /// <exception cref="TLException">Thrown if compilation fails.</exception>
    public void Compile(string path, bool compress = false)
    {
        ThrowIfDisposed();
        var result = NativeMethods.tl_document_compile(_handle, path, compress);
        result.ThrowIfError();
    }

    /// <summary>
    /// Get all schemas defined in the document.
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
    /// <param name="name">The schema name to look up.</param>
    /// <returns>The schema, or null if not found.</returns>
    public TLSchema? GetSchema(string name)
    {
        ThrowIfDisposed();
        LoadSchemas();
        return _schemaMap!.TryGetValue(name, out var schema) ? schema : null;
    }

    /// <summary>
    /// Get the number of schemas in the document.
    /// </summary>
    public int SchemaCount
    {
        get
        {
            ThrowIfDisposed();
            return (int)NativeMethods.tl_document_schema_count(_handle);
        }
    }

    /// <summary>
    /// Check if the document contains a key.
    /// </summary>
    public bool ContainsKey(string key)
    {
        return Get(key) != null;
    }

    /// <inheritdoc/>
    public override string ToString() => ToText();

    private void LoadSchemas()
    {
        if (_schemas != null)
            return;

        var count = (int)NativeMethods.tl_document_schema_count(_handle);
        _schemas = new TLSchema[count];
        _schemaMap = new Dictionary<string, TLSchema>(count);

        for (int i = 0; i < count; i++)
        {
            var namePtr = NativeMethods.tl_document_schema_name(_handle, (nuint)i);
            var name = NativeMethods.PtrToStringAndFree(namePtr) ?? $"schema_{i}";

            var fieldCount = (int)NativeMethods.tl_document_schema_field_count(_handle, (nuint)i);
            var fields = new TLField[fieldCount];

            for (int j = 0; j < fieldCount; j++)
            {
                var fieldNamePtr = NativeMethods.tl_document_schema_field_name(_handle, (nuint)i, (nuint)j);
                var fieldName = NativeMethods.PtrToStringAndFree(fieldNamePtr) ?? $"field_{j}";

                var fieldTypePtr = NativeMethods.tl_document_schema_field_type(_handle, (nuint)i, (nuint)j);
                var fieldType = NativeMethods.PtrToStringAndFree(fieldTypePtr) ?? "unknown";

                var nullable = NativeMethods.tl_document_schema_field_nullable(_handle, (nuint)i, (nuint)j);
                var isArray = NativeMethods.tl_document_schema_field_is_array(_handle, (nuint)i, (nuint)j);

                fields[j] = new TLField(fieldName, fieldType, nullable, isArray);
            }

            var schema = new TLSchema(name, fields);
            _schemas[i] = schema;
            _schemaMap[name] = schema;
        }
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(TLDocument));
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (!_disposed && _handle != IntPtr.Zero)
        {
            NativeMethods.tl_document_free(_handle);
            _handle = IntPtr.Zero;
            _disposed = true;
        }
    }
}
