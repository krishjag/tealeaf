namespace TeaLeaf.Annotations;

/// <summary>
/// Marks a field as optional (nullable) in the TeaLeaf schema.
/// For nullable reference types (string?), this is inferred automatically.
/// Use this attribute on non-nullable types to force nullable schema output.
/// </summary>
[AttributeUsage(AttributeTargets.Property | AttributeTargets.Field, AllowMultiple = false)]
public sealed class TLOptionalAttribute : Attribute
{
}
