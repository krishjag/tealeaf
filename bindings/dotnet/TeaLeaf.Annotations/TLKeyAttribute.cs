namespace TeaLeaf.Annotations;

/// <summary>
/// Specifies the top-level key name when this DTO is serialized as
/// a document-level entry. If not specified, a snake_case version
/// of the class name is used.
/// </summary>
[AttributeUsage(AttributeTargets.Class | AttributeTargets.Struct, AllowMultiple = false)]
public sealed class TLKeyAttribute : Attribute
{
    /// <summary>
    /// The top-level key name used when serializing this type as a document entry.
    /// </summary>
    public string Key { get; }

    /// <summary>
    /// Initializes a new instance of <see cref="TLKeyAttribute"/> with the specified key.
    /// </summary>
    /// <param name="key">The top-level key name to use in the document.</param>
    public TLKeyAttribute(string key)
    {
        Key = key;
    }
}
