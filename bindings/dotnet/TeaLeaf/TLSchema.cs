namespace TeaLeaf;

/// <summary>
/// Represents a field in a TeaLeaf schema.
/// </summary>
public sealed class TLField
{
    /// <summary>
    /// The name of the field.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// The base type of the field (e.g., "int", "string", "user").
    /// </summary>
    public string Type { get; }

    /// <summary>
    /// Whether the field can be null.
    /// </summary>
    public bool IsNullable { get; }

    /// <summary>
    /// Whether the field is an array type.
    /// </summary>
    public bool IsArray { get; }

    internal TLField(string name, string type, bool nullable, bool isArray)
    {
        Name = name;
        Type = type;
        IsNullable = nullable;
        IsArray = isArray;
    }

    public override string ToString() =>
        $"{Name}: {Type}{(IsArray ? "[]" : "")}{(IsNullable ? "?" : "")}";
}

/// <summary>
/// Represents a schema definition from a TeaLeaf file.
/// </summary>
public sealed class TLSchema
{
    /// <summary>
    /// The name of the schema (struct type).
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// The fields defined in this schema.
    /// </summary>
    public IReadOnlyList<TLField> Fields { get; }

    internal TLSchema(string name, IReadOnlyList<TLField> fields)
    {
        Name = name;
        Fields = fields;
    }

    /// <summary>
    /// Get a field by name.
    /// </summary>
    public TLField? GetField(string name) =>
        Fields.FirstOrDefault(f => f.Name == name);

    /// <summary>
    /// Check if the schema contains a field with the given name.
    /// </summary>
    public bool HasField(string name) =>
        Fields.Any(f => f.Name == name);

    public override string ToString() =>
        $"@struct {Name} ({string.Join(", ", Fields)})";
}
