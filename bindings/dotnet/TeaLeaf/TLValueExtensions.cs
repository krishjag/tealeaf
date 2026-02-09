namespace TeaLeaf;

/// <summary>
/// Extension methods for <see cref="TLValue"/> and <see cref="TLDocument"/> that provide
/// non-nullable access patterns, reducing CS8602 warnings in consuming code.
/// </summary>
public static class TLValueExtensions
{
    /// <summary>
    /// Gets a required field from this object value by key. Throws if not found.
    /// </summary>
    /// <param name="value">The TLValue to access (must be an object type).</param>
    /// <param name="key">The field name to look up.</param>
    /// <returns>The field value as a non-nullable TLValue. The caller must dispose.</returns>
    /// <exception cref="KeyNotFoundException">Thrown when the key is not found or the value is not an object.</exception>
    public static TLValue GetRequired(this TLValue value, string key)
    {
        var result = value[key];
        if (result is null)
            throw new KeyNotFoundException($"Required key '{key}' not found in TeaLeaf value.");
        return result;
    }

    /// <summary>
    /// Gets a required array element by index. Throws if not found.
    /// </summary>
    /// <param name="value">The TLValue to access (must be an array type).</param>
    /// <param name="index">The zero-based index of the element.</param>
    /// <returns>The element as a non-nullable TLValue. The caller must dispose.</returns>
    /// <exception cref="IndexOutOfRangeException">Thrown when the index is out of bounds or the value is not an array.</exception>
    public static TLValue GetRequired(this TLValue value, int index)
    {
        var result = value[index];
        if (result is null)
            throw new IndexOutOfRangeException($"Required index {index} not found in TeaLeaf array value.");
        return result;
    }

    /// <summary>
    /// Gets a required value from this document by key. Throws if not found.
    /// </summary>
    /// <param name="doc">The TLDocument to access.</param>
    /// <param name="key">The top-level key to look up.</param>
    /// <returns>The value as a non-nullable TLValue. The caller must dispose.</returns>
    /// <exception cref="KeyNotFoundException">Thrown when the key is not found in the document.</exception>
    public static TLValue GetRequired(this TLDocument doc, string key)
    {
        var result = doc[key];
        if (result is null)
            throw new KeyNotFoundException($"Required key '{key}' not found in TeaLeaf document.");
        return result;
    }
}
