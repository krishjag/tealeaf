namespace TeaLeaf.Annotations;

/// <summary>
/// Overrides the field name used in TeaLeaf output.
/// </summary>
[AttributeUsage(AttributeTargets.Property | AttributeTargets.Field, AllowMultiple = false)]
public sealed class TLRenameAttribute : Attribute
{
    /// <summary>
    /// The custom field name to use in TeaLeaf output instead of the default snake_case conversion.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// Initializes a new instance of <see cref="TLRenameAttribute"/> with the specified field name.
    /// </summary>
    /// <param name="name">The custom field name to use in TeaLeaf output.</param>
    public TLRenameAttribute(string name)
    {
        Name = name;
    }
}
