using System.Collections.Immutable;
using System.Reflection;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Xunit;

namespace TeaLeaf.Generators.Tests;

public class GeneratorTests
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

    [Fact]
    public void Generator_SimpleClass_GeneratesSerializationMethods()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class User
{
    public string Name { get; set; } = """";
    public int Age { get; set; }
    public bool Active { get; set; }
}
";
        var result = RunGenerator(source);

        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));
        Assert.Single(result.GeneratedTrees.Where(t => t.FilePath.Contains("User.TeaLeaf.g.cs")));

        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("User.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        // Verify serialization methods
        Assert.Contains("ToTeaLeafText()", generatedSource);
        Assert.Contains("WriteTeaLeafObjectBody(", generatedSource);
        Assert.Contains("ToTeaLeafDocument(", generatedSource);
        Assert.Contains("ToTLDocument(", generatedSource);
        Assert.Contains("ToTeaLeafJson(", generatedSource);
        Assert.Contains("CompileToTeaLeaf(", generatedSource);
        Assert.Contains("GetTeaLeafSchema()", generatedSource);

        // Verify deserialization methods
        Assert.Contains("FromTeaLeaf(TeaLeaf.TLDocument", generatedSource);
        Assert.Contains("FromTeaLeaf(TeaLeaf.TLValue", generatedSource);
    }

    [Fact]
    public void Generator_EmitsRuntimeHelper()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class Simple
{
    public string Value { get; set; } = """";
}
";
        var result = RunGenerator(source);

        Assert.Single(result.GeneratedTrees.Where(t => t.FilePath.Contains("TLTextHelper.g.cs")));

        var helperSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("TLTextHelper.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("class TLTextHelper", helperSource);
        Assert.Contains("AppendString", helperSource);
        Assert.Contains("AppendValue", helperSource);
        Assert.Contains("ToSnakeCase", helperSource);
    }

    [Fact]
    public void Generator_SchemaContainsCorrectFieldTypes()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class TypedModel
{
    public string Name { get; set; } = """";
    public int Count { get; set; }
    public double Price { get; set; }
    public bool Active { get; set; }
    public long Timestamp { get; set; }
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("TypedModel.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        // Schema should contain correct TeaLeaf types
        Assert.Contains("@struct typed_model", generatedSource);
        Assert.Contains("name: string", generatedSource);
        Assert.Contains("count: int", generatedSource);
        Assert.Contains("price: float", generatedSource);
        Assert.Contains("active: bool", generatedSource);
        Assert.Contains("timestamp: int64", generatedSource);
    }

    [Fact]
    public void Generator_RenameAttribute_UsesCustomName()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(StructName = ""my_user"")]
public partial class UserRecord
{
    [TLRename(""user_name"")]
    public string Name { get; set; } = """";

    [TLRename(""user_email"")]
    public string Email { get; set; } = """";
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("UserRecord.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("@struct my_user", generatedSource);
        Assert.Contains("user_name: string", generatedSource);
        Assert.Contains("user_email: string", generatedSource);
    }

    [Fact]
    public void Generator_SkipAttribute_ExcludesProperty()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class WithSkipped
{
    public string Name { get; set; } = """";

    [TLSkip]
    public string InternalId { get; set; } = """";

    public int Age { get; set; }
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithSkipped.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        // Name and Age should be present, InternalId should not
        Assert.Contains("name: string", generatedSource);
        Assert.Contains("age: int", generatedSource);
        Assert.DoesNotContain("internal_id", generatedSource);
    }

    [Fact]
    public void Generator_NullableProperty_MarkedAsOptional()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class WithNullable
{
    public string Name { get; set; } = """";
    public string? Email { get; set; }
    public int? Age { get; set; }
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithNullable.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        // Nullable fields should have ? in schema
        Assert.Contains("email: string?", generatedSource);
        Assert.Contains("age: int?", generatedSource);
        // Non-nullable should not
        Assert.Contains("name: string", generatedSource);
    }

    [Fact]
    public void Generator_TLOptionalAttribute_ForcesNullable()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class WithOptional
{
    public string Name { get; set; } = """";

    [TLOptional]
    public int Score { get; set; }
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithOptional.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("score: int?", generatedSource);
    }

    [Fact]
    public void Generator_TLKeyAttribute_OverridesDefaultKey()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
[TLKey(""my_data"")]
public partial class CustomKey
{
    public string Value { get; set; } = """";
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("CustomKey.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("key = \"my_data\"", generatedSource);
    }

    [Fact]
    public void Generator_EnumProperty_MappedToString()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

public enum Status { Active, Inactive, Pending }

[TeaLeaf]
public partial class WithEnum
{
    public string Name { get; set; } = """";
    public Status Status { get; set; }
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithEnum.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("status: string", generatedSource);
        Assert.Contains("ToSnakeCase", generatedSource);
    }

    [Fact]
    public void Generator_ListProperty_GeneratesArrayHandling()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class WithList
{
    public string Name { get; set; } = """";
    public List<string> Tags { get; set; } = new();
    public List<int> Scores { get; set; } = new();
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithList.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("tags: []string", generatedSource);
        Assert.Contains("scores: []int", generatedSource);
        // Serialization should emit array
        Assert.Contains("tags: [", generatedSource);
        // Deserialization should create list
        Assert.Contains("List<string>", generatedSource);
        Assert.Contains("List<int>", generatedSource);
    }

    [Fact]
    public void Generator_DictionaryProperty_GeneratesObjectHandling()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class WithDict
{
    public string Name { get; set; } = """";
    public Dictionary<string, string> Metadata { get; set; } = new();
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithDict.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("metadata: object", generatedSource);
        Assert.Contains("Dictionary<string, string>", generatedSource);
    }

    [Fact]
    public void Generator_NamespaceIsPreserved()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace My.App.Models;

[TeaLeaf]
public partial class Config
{
    public string Value { get; set; } = """";
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Config.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("namespace My.App.Models;", generatedSource);
    }

    [Fact]
    public void Generator_BoolProperty_SerializesAsTrueFalse()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class BoolModel
{
    public bool Enabled { get; set; }
    public bool Visible { get; set; }
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("BoolModel.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        // Bool serialization should use true/false literals
        Assert.Contains("\"true\" : \"false\"", generatedSource);
    }

    [Fact]
    public void Generator_MultipleClasses_GeneratesForEach()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class ModelA
{
    public string Name { get; set; } = """";
}

[TeaLeaf]
public partial class ModelB
{
    public int Value { get; set; }
}
";
        var result = RunGenerator(source);

        Assert.Single(result.GeneratedTrees.Where(t => t.FilePath.Contains("ModelA.TeaLeaf.g.cs")));
        Assert.Single(result.GeneratedTrees.Where(t => t.FilePath.Contains("ModelB.TeaLeaf.g.cs")));
    }

    [Fact]
    public void Generator_SnakeCaseConversion_IsCorrect()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class CamelCaseModel
{
    public string FirstName { get; set; } = """";
    public string LastName { get; set; } = """";
    public int MaxRetryCount { get; set; }
}
";
        var result = RunGenerator(source);
        var generatedSource = result.GeneratedTrees
            .First(t => t.FilePath.Contains("CamelCaseModel.TeaLeaf.g.cs"))
            .GetText()
            .ToString();

        Assert.Contains("first_name: string", generatedSource);
        Assert.Contains("last_name: string", generatedSource);
        Assert.Contains("max_retry_count: int", generatedSource);
    }

    [Fact]
    public void Generator_OpenGenericType_ReportsTL006Diagnostic()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class Wrapper<T>
{
    public string Label { get; set; } = """";
    public T? Value { get; set; }
}
";
        var result = RunGenerator(source);

        // Should NOT generate any source for the open generic type
        Assert.Empty(result.GeneratedTrees.Where(t => t.FilePath.Contains("Wrapper.TeaLeaf.g.cs")));

        // Should report TL006 info diagnostic
        var tl006 = result.Diagnostics.Where(d => d.Id == "TL006").ToList();
        Assert.Single(tl006);
        Assert.Equal(DiagnosticSeverity.Info, tl006[0].Severity);
    }

    [Fact]
    public void Generator_ClosedGenericNotAffected_StillGenerates()
    {
        // A non-generic class should still generate normally
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class ConcreteModel
{
    public string Name { get; set; } = """";
    public int Count { get; set; }
}
";
        var result = RunGenerator(source);

        Assert.Single(result.GeneratedTrees.Where(t => t.FilePath.Contains("ConcreteModel.TeaLeaf.g.cs")));
        Assert.Empty(result.Diagnostics.Where(d => d.Id == "TL006"));
    }
}
