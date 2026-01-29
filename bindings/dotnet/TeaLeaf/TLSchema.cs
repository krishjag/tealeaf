namespace Pax;

/// <summary>
/// Represents a field in a Pax schema.
/// </summary>
public sealed class PaxField
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

    internal PaxField(string name, string type, bool nullable, bool isArray)
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
/// Represents a schema definition from a Pax file.
/// </summary>
public sealed class PaxSchema
{
    /// <summary>
    /// The name of the schema (struct type).
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// The fields defined in this schema.
    /// </summary>
    public IReadOnlyList<PaxField> Fields { get; }

    internal PaxSchema(string name, IReadOnlyList<PaxField> fields)
    {
        Name = name;
        Fields = fields;
    }

    /// <summary>
    /// Get a field by name.
    /// </summary>
    public PaxField? GetField(string name) =>
        Fields.FirstOrDefault(f => f.Name == name);

    /// <summary>
    /// Check if the schema contains a field with the given name.
    /// </summary>
    public bool HasField(string name) =>
        Fields.Any(f => f.Name == name);

    public override string ToString() =>
        $"@struct {Name} ({string.Join(", ", Fields)})";
}
