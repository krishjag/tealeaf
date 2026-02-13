using System.Collections.Immutable;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Xunit;

namespace TeaLeaf.Generators.Tests;

public class DiagnosticTests
{
    private static GeneratorDriverRunResult RunGenerator(string source)
    {
        var syntaxTree = CSharpSyntaxTree.ParseText(source);

        var references = new List<MetadataReference>
        {
            MetadataReference.CreateFromFile(typeof(object).Assembly.Location),
            MetadataReference.CreateFromFile(typeof(Attribute).Assembly.Location),
            MetadataReference.CreateFromFile(typeof(Annotations.TeaLeafAttribute).Assembly.Location),
        };

        // Add System.Runtime reference
        var runtimeDir = Path.GetDirectoryName(typeof(object).Assembly.Location)!;
        var runtimeRef = Path.Combine(runtimeDir, "System.Runtime.dll");
        if (File.Exists(runtimeRef))
            references.Add(MetadataReference.CreateFromFile(runtimeRef));

        var compilation = CSharpCompilation.Create(
            "TestAssembly",
            new[] { syntaxTree },
            references,
            new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary)
                .WithNullableContextOptions(NullableContextOptions.Enable));

        var generator = new TeaLeafGenerator();
        GeneratorDriver driver = CSharpGeneratorDriver.Create(generator);
        driver = driver.RunGeneratorsAndUpdateCompilation(compilation, out var outputCompilation, out var diagnostics);

        return driver.GetRunResult();
    }

    // =========================================================================
    // TL001: Non-partial class
    // =========================================================================

    [Fact]
    public void NonPartialClass_ReportsTL001Diagnostic()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public class NotPartial
{
    public string Name { get; set; } = """";
}
";
        var result = RunGenerator(source);

        var tl001 = result.Diagnostics.Where(d => d.Id == "TL001").ToList();
        Assert.Single(tl001);
        Assert.Equal(DiagnosticSeverity.Error, tl001[0].Severity);
        Assert.Contains("NotPartial", tl001[0].GetMessage());
    }

    [Fact]
    public void PartialClass_DoesNotReportTL001()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class IsPartial
{
    public string Name { get; set; } = """";
}
";
        var result = RunGenerator(source);

        var tl001 = result.Diagnostics.Where(d => d.Id == "TL001").ToList();
        Assert.Empty(tl001);
    }

    // =========================================================================
    // TL003: Invalid TLType value
    // =========================================================================

    [Fact]
    public void InvalidTLType_ReportsTL003Diagnostic()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithInvalidType
{
    [TLType(""bogus_type"")]
    public string Name { get; set; } = """";
}
";
        var result = RunGenerator(source);

        var tl003 = result.Diagnostics.Where(d => d.Id == "TL003").ToList();
        Assert.Single(tl003);
        Assert.Equal(DiagnosticSeverity.Error, tl003[0].Severity);
        Assert.Contains("bogus_type", tl003[0].GetMessage());
    }

    [Fact]
    public void ValidTLType_DoesNotReportTL003()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithValidType
{
    [TLType(""timestamp"")]
    public long CreatedAt { get; set; }
}
";
        var result = RunGenerator(source);

        var tl003 = result.Diagnostics.Where(d => d.Id == "TL003").ToList();
        Assert.Empty(tl003);
    }

    // =========================================================================
    // TL006: Open generic type
    // =========================================================================

    [Fact]
    public void OpenGenericType_ReportsTL006Diagnostic()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class Container<T>
{
    public string Label { get; set; } = """";
    public T? Inner { get; set; }
}
";
        var result = RunGenerator(source);

        var tl006 = result.Diagnostics.Where(d => d.Id == "TL006").ToList();
        Assert.Single(tl006);
        Assert.Equal(DiagnosticSeverity.Info, tl006[0].Severity);
        Assert.Contains("Container", tl006[0].GetMessage());

        // Should NOT generate code for the open generic type
        Assert.Empty(result.GeneratedTrees.Where(t => t.FilePath.Contains("Container.TeaLeaf.g.cs")));
    }

    // =========================================================================
    // TL004: Nested type not annotated with [TeaLeaf]
    // =========================================================================

    [Fact]
    public void NestedTypeNotAnnotated_ReportsTL004Diagnostic()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

public class PlainAddress
{
    public string Street { get; set; } = """";
    public string City { get; set; } = """";
}

[TeaLeaf(Generate = true)]
public partial class PersonWithPlainAddress
{
    public string Name { get; set; } = """";
    public PlainAddress Home { get; set; } = new();
}
";
        var result = RunGenerator(source);

        var tl004 = result.Diagnostics.Where(d => d.Id == "TL004").ToList();
        Assert.Single(tl004);
        Assert.Equal(DiagnosticSeverity.Info, tl004[0].Severity);
        Assert.Contains("Home", tl004[0].GetMessage());
    }

    // =========================================================================
    // Various property kinds in generated code
    // =========================================================================

    [Fact]
    public void Generator_TimestampOverride_GeneratesCorrectly()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class Event
{
    public string Name { get; set; } = """";

    [TLType(""timestamp"")]
    public long CreatedAt { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Event.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("created_at: timestamp", generatedSource);
    }

    [Fact]
    public void Generator_NullableListProperty_GeneratesArrayHandling()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithNullableList
{
    public string Name { get; set; } = """";
    public List<double>? Scores { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithNullableList.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("scores:", generatedSource);
    }

    [Fact]
    public void Generator_ByteArrayProperty_MappedToBytes()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class BinaryData
{
    public string Label { get; set; } = """";
    public byte[]? Payload { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("BinaryData.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("payload: bytes", generatedSource);
    }

    [Fact]
    public void Generator_MultipleAttributes_Combined()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true, StructName = ""my_config"")]
[TLKey(""app_config"")]
public partial class Config
{
    [TLRename(""app_name"")]
    public string Name { get; set; } = """";

    [TLSkip]
    public string Internal { get; set; } = """";

    [TLOptional]
    public int Score { get; set; }

    public string? Description { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Config.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("@struct my_config", generatedSource);
        Assert.Contains("app_name: string", generatedSource);
        Assert.DoesNotContain("internal", generatedSource.ToLower().Replace("internal_", ""));
        Assert.Contains("score: int?", generatedSource);
        Assert.Contains("description: string?", generatedSource);
        Assert.Contains("key = \"app_config\"", generatedSource);
    }
}
