namespace TeaLeaf.Annotations;

/// <summary>
/// Excludes a property from TeaLeaf serialization and deserialization.
/// </summary>
[AttributeUsage(AttributeTargets.Property | AttributeTargets.Field, AllowMultiple = false)]
public sealed class TLSkipAttribute : Attribute
{
}
