namespace TeaLeaf.Annotations;

/// <summary>
/// Marks a class or record for TeaLeaf source generation.
/// The generator will create serialization (ToTeaLeaf*) and
/// deserialization (FromTeaLeaf) methods as partial extensions.
/// </summary>
[AttributeUsage(AttributeTargets.Class | AttributeTargets.Struct, AllowMultiple = false, Inherited = false)]
public sealed class TeaLeafAttribute : Attribute
{
    /// <summary>
    /// Optional: override the struct name used in @struct definitions.
    /// Defaults to a snake_case version of the class name.
    /// </summary>
    public string? StructName { get; set; }

    /// <summary>
    /// If true, generates @struct and @table output for arrays of this type.
    /// Defaults to true.
    /// </summary>
    public bool EmitSchema { get; set; } = true;
}
