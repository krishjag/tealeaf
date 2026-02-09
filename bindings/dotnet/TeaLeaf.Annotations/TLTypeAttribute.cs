namespace TeaLeaf.Annotations;

/// <summary>
/// Overrides the TeaLeaf type used for a field in schema generation.
/// Valid types: "bool", "int", "int8", "int16", "int32", "int64",
/// "uint", "uint8", "uint16", "uint32", "uint64",
/// "float", "float32", "float64", "string", "bytes", "timestamp".
/// </summary>
[AttributeUsage(AttributeTargets.Property | AttributeTargets.Field, AllowMultiple = false)]
public sealed class TLTypeAttribute : Attribute
{
    /// <summary>
    /// The TeaLeaf type name to use in schema generation for this field.
    /// </summary>
    public string TypeName { get; }

    /// <summary>
    /// Initializes a new instance of <see cref="TLTypeAttribute"/> with the specified type name.
    /// </summary>
    /// <param name="typeName">The TeaLeaf type name (e.g., "timestamp", "int64", "bytes").</param>
    public TLTypeAttribute(string typeName)
    {
        TypeName = typeName;
    }
}
