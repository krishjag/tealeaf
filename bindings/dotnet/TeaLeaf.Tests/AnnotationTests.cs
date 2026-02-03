using TeaLeaf.Annotations;
using Xunit;

namespace TeaLeaf.Tests;

/// <summary>
/// Tests that all annotation attribute types can be constructed and
/// their properties read back correctly.
/// </summary>
public class AnnotationTests
{
    // =========================================================================
    // TeaLeafAttribute
    // =========================================================================

    [Fact]
    public void TeaLeafAttribute_DefaultValues()
    {
        var attr = new TeaLeafAttribute();
        Assert.Null(attr.StructName);
        Assert.True(attr.EmitSchema);
    }

    [Fact]
    public void TeaLeafAttribute_CustomStructName()
    {
        var attr = new TeaLeafAttribute { StructName = "custom_name" };
        Assert.Equal("custom_name", attr.StructName);
    }

    [Fact]
    public void TeaLeafAttribute_EmitSchemaFalse()
    {
        var attr = new TeaLeafAttribute { EmitSchema = false };
        Assert.False(attr.EmitSchema);
    }

    // =========================================================================
    // TLKeyAttribute
    // =========================================================================

    [Fact]
    public void TLKeyAttribute_StoresKey()
    {
        var attr = new TLKeyAttribute("my_key");
        Assert.Equal("my_key", attr.Key);
    }

    [Fact]
    public void TLKeyAttribute_EmptyKey()
    {
        var attr = new TLKeyAttribute("");
        Assert.Equal("", attr.Key);
    }

    [Fact]
    public void TLKeyAttribute_SpecialCharKey()
    {
        var attr = new TLKeyAttribute("my-special.key_1");
        Assert.Equal("my-special.key_1", attr.Key);
    }

    // =========================================================================
    // TLRenameAttribute
    // =========================================================================

    [Fact]
    public void TLRenameAttribute_StoresName()
    {
        var attr = new TLRenameAttribute("custom_field");
        Assert.Equal("custom_field", attr.Name);
    }

    [Fact]
    public void TLRenameAttribute_EmptyName()
    {
        var attr = new TLRenameAttribute("");
        Assert.Equal("", attr.Name);
    }

    [Fact]
    public void TLRenameAttribute_SpecialChars()
    {
        var attr = new TLRenameAttribute("field-with.dots_and-dashes");
        Assert.Equal("field-with.dots_and-dashes", attr.Name);
    }

    // =========================================================================
    // TLTypeAttribute
    // =========================================================================

    [Fact]
    public void TLTypeAttribute_StoresTypeName()
    {
        var attr = new TLTypeAttribute("timestamp");
        Assert.Equal("timestamp", attr.TypeName);
    }

    [Fact]
    public void TLTypeAttribute_AllValidTypes()
    {
        var validTypes = new[]
        {
            "bool", "int", "int8", "int16", "int32", "int64",
            "uint", "uint8", "uint16", "uint32", "uint64",
            "float", "float32", "float64", "string", "bytes", "timestamp"
        };

        foreach (var typeName in validTypes)
        {
            var attr = new TLTypeAttribute(typeName);
            Assert.Equal(typeName, attr.TypeName);
        }
    }

    // =========================================================================
    // TLSkipAttribute
    // =========================================================================

    [Fact]
    public void TLSkipAttribute_CanConstruct()
    {
        var attr = new TLSkipAttribute();
        Assert.NotNull(attr);
    }

    // =========================================================================
    // TLOptionalAttribute
    // =========================================================================

    [Fact]
    public void TLOptionalAttribute_CanConstruct()
    {
        var attr = new TLOptionalAttribute();
        Assert.NotNull(attr);
    }
}
