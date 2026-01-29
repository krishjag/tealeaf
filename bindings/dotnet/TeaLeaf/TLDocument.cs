using Pax.Native;

namespace Pax;

/// <summary>
/// Represents a parsed Pax document.
/// </summary>
public sealed class PaxDocument : IDisposable
{
    private IntPtr _handle;
    private bool _disposed;

    private PaxDocument(IntPtr handle)
    {
        _handle = handle;
    }

    /// <summary>
    /// Parse a Pax document from a text string.
    /// </summary>
    /// <param name="text">The Pax text to parse.</param>
    /// <returns>A parsed PaxDocument.</returns>
    /// <exception cref="PaxException">Thrown if parsing fails.</exception>
    public static PaxDocument Parse(string text)
    {
        var handle = NativeMethods.pax_parse(text);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? "Failed to parse Pax document";
            throw new PaxException(error);
        }
        return new PaxDocument(handle);
    }

    /// <summary>
    /// Parse a Pax document from a file.
    /// </summary>
    /// <param name="path">Path to the .pax file.</param>
    /// <returns>A parsed PaxDocument.</returns>
    /// <exception cref="PaxException">Thrown if parsing fails.</exception>
    public static PaxDocument ParseFile(string path)
    {
        var handle = NativeMethods.pax_parse_file(path);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? $"Failed to parse Pax file: {path}";
            throw new PaxException(error);
        }
        return new PaxDocument(handle);
    }

    /// <summary>
    /// Try to parse a Pax document from text.
    /// </summary>
    /// <param name="text">The Pax text to parse.</param>
    /// <param name="document">The parsed document, if successful.</param>
    /// <returns>True if parsing succeeded.</returns>
    public static bool TryParse(string text, out PaxDocument? document)
    {
        var handle = NativeMethods.pax_parse(text);
        if (handle == IntPtr.Zero)
        {
            document = null;
            return false;
        }
        document = new PaxDocument(handle);
        return true;
    }

    /// <summary>
    /// Create a Pax document from a JSON string with automatic schema inference.
    /// Uniform object arrays are converted to @struct definitions and @table format.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <returns>A PaxDocument containing the JSON data with inferred schemas.</returns>
    /// <exception cref="PaxException">Thrown if JSON parsing fails.</exception>
    public static PaxDocument FromJson(string json)
    {
        var handle = NativeMethods.pax_document_from_json(json);
        if (handle == IntPtr.Zero)
        {
            var error = NativeMethods.GetLastError() ?? "Failed to parse JSON";
            throw new PaxException(error);
        }
        return new PaxDocument(handle);
    }

    /// <summary>
    /// Try to create a Pax document from a JSON string with schema inference.
    /// </summary>
    /// <param name="json">The JSON string to convert.</param>
    /// <param name="document">The parsed document, if successful.</param>
    /// <returns>True if parsing succeeded.</returns>
    public static bool TryFromJson(string json, out PaxDocument? document)
    {
        var handle = NativeMethods.pax_document_from_json(json);
        if (handle == IntPtr.Zero)
        {
            document = null;
            return false;
        }
        document = new PaxDocument(handle);
        return true;
    }

    /// <summary>
    /// Get a value from the document by key.
    /// </summary>
    /// <param name="key">The key to look up.</param>
    /// <returns>The value, or null if not found.</returns>
    public PaxValue? Get(string key)
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.pax_document_get(_handle, key);
        return ptr == IntPtr.Zero ? null : new PaxValue(ptr);
    }

    /// <summary>
    /// Get all keys in the document.
    /// </summary>
    public string[] Keys
    {
        get
        {
            ThrowIfDisposed();
            var ptr = NativeMethods.pax_document_keys(_handle);
            return NativeMethods.PtrToStringArrayAndFree(ptr);
        }
    }

    /// <summary>
    /// Indexer for key-based access.
    /// </summary>
    public PaxValue? this[string key] => Get(key);

    /// <summary>
    /// Convert the document to Pax text format with schema definitions.
    /// This is the default format that includes @struct definitions at the top.
    /// </summary>
    public string ToText()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.pax_document_to_text(_handle);
        return NativeMethods.PtrToStringAndFree(ptr) ?? string.Empty;
    }

    /// <summary>
    /// Convert the document to Pax text format without schema definitions (data only).
    /// Use this when you only want the data portion without @struct definitions.
    /// </summary>
    public string ToTextDataOnly()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.pax_document_to_text_data_only(_handle);
        return NativeMethods.PtrToStringAndFree(ptr) ?? string.Empty;
    }

    /// <summary>
    /// Convert the document to pretty-printed JSON.
    /// </summary>
    /// <returns>A JSON string representation of the document.</returns>
    public string ToJson()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.pax_document_to_json(_handle);
        return NativeMethods.PtrToStringAndFree(ptr) ?? "{}";
    }

    /// <summary>
    /// Convert the document to compact JSON (no extra whitespace).
    /// </summary>
    /// <returns>A compact JSON string representation of the document.</returns>
    public string ToJsonCompact()
    {
        ThrowIfDisposed();
        var ptr = NativeMethods.pax_document_to_json_compact(_handle);
        return NativeMethods.PtrToStringAndFree(ptr) ?? "{}";
    }

    /// <summary>
    /// Compile the document to binary format and write to a file.
    /// </summary>
    /// <param name="path">Output file path (.paxb).</param>
    /// <param name="compress">Whether to compress the output.</param>
    /// <exception cref="PaxException">Thrown if compilation fails.</exception>
    public void Compile(string path, bool compress = false)
    {
        ThrowIfDisposed();
        var result = NativeMethods.pax_document_compile(_handle, path, compress);
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
            throw new ObjectDisposedException(nameof(PaxDocument));
    }

    public void Dispose()
    {
        if (!_disposed && _handle != IntPtr.Zero)
        {
            NativeMethods.pax_document_free(_handle);
            _handle = IntPtr.Zero;
            _disposed = true;
        }
    }
}
