namespace TeaLeaf.Annotations;

/// <summary>
/// Marks a class or record for TeaLeaf serialization.
/// Used by the reflection-based TeaLeafSerializer for runtime serialization.
/// Set <see cref="Generate"/> to true to enable compile-time source generation
/// (requires the class to be declared as partial).
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

    /// <summary>
    /// If true, the source generator will generate serialization and deserialization methods.
    /// Requires the class to be declared as partial. Defaults to false (reflection-only).
    /// </summary>
    public bool Generate { get; set; } = false;
}
