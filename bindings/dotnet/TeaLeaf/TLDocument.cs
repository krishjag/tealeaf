using TeaLeaf.Native;

namespace TeaLeaf;

/// <summary>
/// Represents a parsed TeaLeaf document.
/// </summary>
public sealed class TLDocument : IDisposable
{
    private IntPtr _handle;
    private bool _disposed;

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
    /// Convert the document to TeaLeaf text format with schema definitions.
    /// This is the default format that includes @struct definitions at the top.
    /// </summary>
    public string ToText()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.tl_document_to_text(_handle);
        return NativeMethods.PtrToStringAndFree(ptr) ?? string.Empty;
    }

    /// <summary>
    /// Convert the document to TeaLeaf text format without schema definitions (data only).
    /// Use this when you only want the data portion without @struct definitions.
    /// </summary>
    public string ToTextDataOnly()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.tl_document_to_text_data_only(_handle);
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
    /// Check if the document contains a key.
    /// </summary>
    public bool ContainsKey(string key)
    {
        return Get(key) != null;
    }

    public override string ToString() => ToText();

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(TLDocument));
    }

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
