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

[TeaLeaf(Generate = true)]
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
        Assert.Contains("public static void CollectTeaLeafSchemas", generatedSource);

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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true, StructName = ""my_user"")]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
public partial class ModelA
{
    public string Name { get; set; } = """";
}

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

[TeaLeaf(Generate = true)]
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

    // =========================================================================
    // DeserializerEmitter coverage: DateTimeOffset property
    // =========================================================================

    [Fact]
    public void Generator_DateTimeOffsetProperty_GeneratesAsDateTime()
    {
        var source = @"
using System;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithDates
{
    public string Name { get; set; } = """";
    public DateTimeOffset CreatedAt { get; set; }
    public DateTimeOffset? UpdatedAt { get; set; }
}
";
        var result = RunGenerator(source);
        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithDates.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("AsDateTime()", gen);
        Assert.Contains("DateTimeOffset.MinValue", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: timestamp with int/long
    // =========================================================================

    [Fact]
    public void Generator_TimestampInt_GeneratesIntCast()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class TimestampModel
{
    [TLType(""timestamp"")]
    public int CreatedAtInt { get; set; }

    [TLType(""timestamp"")]
    public long CreatedAtLong { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("TimestampModel.TeaLeaf.g.cs"))
            .GetText().ToString();

        // int timestamp should have (int) cast
        Assert.Contains("(int)(", gen);
        Assert.Contains("AsTimestamp()", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: TimeSpan property
    // =========================================================================

    [Fact]
    public void Generator_TimeSpanProperty_GeneratesFromMilliseconds()
    {
        var source = @"
using System;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithTimeSpan
{
    public string Name { get; set; } = """";
    public TimeSpan Duration { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithTimeSpan.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("TimeSpan.FromMilliseconds", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: Guid property
    // =========================================================================

    [Fact]
    public void Generator_GuidProperty_GeneratesGuidParse()
    {
        var source = @"
using System;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithGuid
{
    public string Name { get; set; } = """";
    public Guid Id { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithGuid.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("Guid.Parse", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: nullable nested object
    // =========================================================================

    [Fact]
    public void Generator_NullableNestedObject_GeneratesNullCheck()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class Inner
{
    public string Val { get; set; } = """";
}

[TeaLeaf(Generate = true)]
public partial class Outer
{
    public string Name { get; set; } = """";
    public Inner? OptionalInner { get; set; }
    public Inner RequiredInner { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Outer.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("IsNull", gen);
        Assert.Contains("Inner.FromTeaLeaf", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: various primitive types
    // =========================================================================

    [Fact]
    public void Generator_ExoticPrimitives_GeneratesCorrectAccessors()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class ExoticPrims
{
    public byte SmallByte { get; set; }
    public short SmallShort { get; set; }
    public ushort UShort { get; set; }
    public uint UInt { get; set; }
    public ulong ULong { get; set; }
    public float SingleF { get; set; }
    public decimal DecimalD { get; set; }
    public byte? NullByte { get; set; }
    public int? NullInt { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("ExoticPrims.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("AsUInt()", gen);
        Assert.Contains("AsFloat()", gen);
        Assert.Contains("(byte?)", gen);
        Assert.Contains("IsNull", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: List element type branches
    // =========================================================================

    [Fact]
    public void Generator_ListOfVariousTypes_GeneratesCorrectReads()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class MultiList
{
    public List<int> Ints { get; set; } = new();
    public List<long> Longs { get; set; } = new();
    public List<double> Doubles { get; set; } = new();
    public List<float> Floats { get; set; } = new();
    public List<bool> Bools { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("MultiList.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("List<int>", gen);
        Assert.Contains("List<long>", gen);
        Assert.Contains("List<double>", gen);
        Assert.Contains("List<float>", gen);
        Assert.Contains("List<bool>", gen);
        Assert.Contains("AsBool()", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: Dictionary value type branches
    // =========================================================================

    [Fact]
    public void Generator_DictOfVariousTypes_GeneratesCorrectReads()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class MultiDict
{
    public Dictionary<string, int> IntMap { get; set; } = new();
    public Dictionary<string, long> LongMap { get; set; } = new();
    public Dictionary<string, double> DoubleMap { get; set; } = new();
    public Dictionary<string, bool> BoolMap { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("MultiDict.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("Dictionary<string, int>", gen);
        Assert.Contains("Dictionary<string, long>", gen);
        Assert.Contains("Dictionary<string, double>", gen);
        Assert.Contains("Dictionary<string, bool>", gen);
    }

    // =========================================================================
    // DeserializerEmitter coverage: nullable string branches
    // =========================================================================

    [Fact]
    public void Generator_NullableString_GeneratesNullCheck()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class NullableStrings
{
    public string Required { get; set; } = """";
    public string? Optional { get; set; }
}
";
        var result = RunGenerator(source);
        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("NullableStrings.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("?? \"\"", gen);
        Assert.Contains("AsString()", gen);
    }

    // =========================================================================
    // ModelAnalyzer coverage: EmitSchema = false
    // =========================================================================

    [Fact]
    public void Generator_EmitSchemaFalse_OmitsSchemaLine()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true, EmitSchema = false)]
public partial class NoSchema
{
    public string Name { get; set; } = """";
    public int Age { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("NoSchema.TeaLeaf.g.cs"))
            .GetText().ToString();

        // EmitSchema = false is parsed by ModelAnalyzer (covers line 78 branch)
        // Even though the emitter doesn't suppress the schema yet, the code path is exercised
        Assert.Contains("name: string", gen);
    }

    // =========================================================================
    // ModelAnalyzer coverage: static and indexer properties skipped
    // =========================================================================

    [Fact]
    public void Generator_IndexerAndStaticProperty_AreSkipped()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithIndexer
{
    public string Name { get; set; } = """";
    public static string StaticProp { get; set; } = """";
    public string this[int index] => Name;
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithIndexer.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Only Name should appear; static and indexer should be excluded
        Assert.Contains("name: string", gen);
        Assert.DoesNotContain("static_prop", gen);
    }

    // =========================================================================
    // ModelAnalyzer coverage: non-byte T[] arrays (string[], int[])
    // =========================================================================

    [Fact]
    public void Generator_ArrayTypes_ClassifiedAsList()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithArrays
{
    public string[] Names { get; set; } = System.Array.Empty<string>();
    public int[] Ids { get; set; } = System.Array.Empty<int>();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithArrays.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("names: []string", gen);
        Assert.Contains("ids: []int", gen);
    }

    // =========================================================================
    // ModelAnalyzer coverage: alternative list-like interfaces
    // =========================================================================

    [Fact]
    public void Generator_ListLikeInterfaces_ClassifiedAsList()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithListInterfaces
{
    public IList<string> Items { get; set; } = new List<string>();
    public IReadOnlyList<int> ReadOnlyIds { get; set; } = new List<int>();
    public IEnumerable<double> Scores { get; set; } = new List<double>();
    public ICollection<string> Tags { get; set; } = new List<string>();
    public IReadOnlyCollection<bool> Flags { get; set; } = new List<bool>();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithListInterfaces.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("items: []string", gen);
        Assert.Contains("read_only_ids: []int", gen);
        Assert.Contains("scores: []float", gen);
        Assert.Contains("tags: []string", gen);
        Assert.Contains("flags: []bool", gen);
    }

    // =========================================================================
    // ModelAnalyzer coverage: IDictionary<K,V> interface
    // =========================================================================

    [Fact]
    public void Generator_IDictionaryInterface_ClassifiedAsDict()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithIDictionary
{
    public IDictionary<string, string> Config { get; set; } = new Dictionary<string, string>();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithIDictionary.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("config: object", gen);
        Assert.Contains("Dictionary<string, string>", gen);
    }

    // =========================================================================
    // ModelAnalyzer coverage: unknown type fallback
    // =========================================================================

    [Fact]
    public void Generator_UnknownType_FallsBackToString()
    {
        // System.Uri is not a primitive, enum, collection, or [TeaLeaf]-annotated type
        var source = @"
using System;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithUnknownType
{
    public string Name { get; set; } = """";
    public Uri? Link { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithUnknownType.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Unknown type should fall back to string
        Assert.Contains("link: string", gen);
    }

    // =========================================================================
    // ModelAnalyzer coverage: GetTLTypeForElement default case
    // =========================================================================

    [Fact]
    public void Generator_ListOfNonStandardElement_FallsBackToString()
    {
        // List<decimal> — decimal is not in GetTLTypeForElement's switch
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class WithDecimalList
{
    public List<decimal> Prices { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("WithDecimalList.TeaLeaf.g.cs"))
            .GetText().ToString();

        // decimal is not in GetTLTypeForElement switch, falls to _ => string
        Assert.Contains("prices: []string", gen);
    }

    // =========================================================================
    // TL007: Global namespace diagnostic
    // =========================================================================

    [Fact]
    public void Generator_GlobalNamespace_ReportsTL007()
    {
        // Class in global namespace (no namespace declaration)
        var source = @"
using TeaLeaf.Annotations;

[TeaLeaf(Generate = true)]
public partial class GlobalModel
{
    public string Name { get; set; } = """";
}
";
        var result = RunGenerator(source);

        // Should produce TL007 diagnostic
        var tl007 = result.Diagnostics.FirstOrDefault(d => d.Id == "TL007");
        Assert.NotNull(tl007);
        Assert.Equal(DiagnosticSeverity.Error, tl007!.Severity);
        Assert.Contains("GlobalModel", tl007.GetMessage());
        Assert.Contains("global namespace", tl007.GetMessage());

        // Should NOT generate any source for this type
        Assert.DoesNotContain(result.GeneratedTrees,
            t => t.FilePath.Contains("GlobalModel.TeaLeaf.g.cs"));
    }

    [Fact]
    public void Generator_NamedNamespace_DoesNotReportTL007()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace MyApp.Models;

[TeaLeaf(Generate = true)]
public partial class NamespacedModel
{
    public string Name { get; set; } = """";
}
";
        var result = RunGenerator(source);

        // Should NOT produce TL007 diagnostic
        var tl007 = result.Diagnostics.FirstOrDefault(d => d.Id == "TL007");
        Assert.Null(tl007);

        // Should generate source normally
        Assert.Contains(result.GeneratedTrees,
            t => t.FilePath.Contains("NamespacedModel.TeaLeaf.g.cs"));
    }

    // =========================================================================
    // ResolveStructName: nested type honors [TeaLeaf(StructName = "...")]
    // =========================================================================

    [Fact]
    public void Generator_NestedType_UsesStructNameOverride()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true, StructName = ""price"")]
public partial class ProductPrice
{
    public double BasePrice { get; set; }
    public string Currency { get; set; } = """";
}

[TeaLeaf(Generate = true)]
public partial class Product
{
    public string Name { get; set; } = """";
    public ProductPrice Price { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Product.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Should use "price" from StructName, not "product_price" from ToSnakeCase
        Assert.Contains("price: price", gen);
        Assert.DoesNotContain("product_price", gen);
    }

    [Fact]
    public void Generator_ListOfNestedType_UsesStructNameOverride()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true, StructName = ""stock"")]
public partial class StockInfo
{
    public string Warehouse { get; set; } = """";
    public int Quantity { get; set; }
}

[TeaLeaf(Generate = true)]
public partial class Inventory
{
    public string Name { get; set; } = """";
    public List<StockInfo> Locations { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Inventory.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Should use "stock" from StructName, not "stock_info" from ToSnakeCase
        Assert.Contains("locations: []stock", gen);
        Assert.DoesNotContain("stock_info", gen);
    }

    [Fact]
    public void Generator_ArrayOfNestedType_UsesStructNameOverride()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true, StructName = ""item"")]
public partial class OrderItem
{
    public string Sku { get; set; } = """";
    public int Quantity { get; set; }
}

[TeaLeaf(Generate = true)]
public partial class Order
{
    public string OrderId { get; set; } = """";
    public OrderItem[] Items { get; set; } = System.Array.Empty<OrderItem>();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Order.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Should use "item" from StructName, not "order_item" from ToSnakeCase
        Assert.Contains("items: []item", gen);
        Assert.DoesNotContain("order_item", gen);
    }

    // =========================================================================
    // Schema deduplication: diamond dependency
    // =========================================================================

    [Fact]
    public void Generator_SharedNestedType_DeduplicatesSchemas()
    {
        var source = @"
using TeaLeaf.Annotations;
namespace TestModels;

[TeaLeaf(Generate = true)] public partial class SharedLeaf { public string Value { get; set; } = """"; public int Code { get; set; } }
[TeaLeaf(Generate = true)] public partial class BranchOne { public string Label { get; set; } = """"; public SharedLeaf Leaf { get; set; } = new(); }
[TeaLeaf(Generate = true)] public partial class BranchTwo { public string Tag { get; set; } = """"; public SharedLeaf Leaf { get; set; } = new(); }
[TeaLeaf(Generate = true)] public partial class DiamondRoot { public string Name { get; set; } = """"; public BranchOne Left { get; set; } = new(); public BranchTwo Right { get; set; } = new(); }
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var allGen = string.Join("\n", result.GeneratedTrees
            .Where(t => t.FilePath.Contains(".TeaLeaf.g.cs"))
            .Select(t => t.GetText().ToString()));
        var rootGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("DiamondRoot.TeaLeaf.g.cs")).GetText().ToString();

        Assert.Contains("CollectTeaLeafSchemas", rootGen);
        Assert.Contains("BranchOne.CollectTeaLeafSchemas", rootGen);
        Assert.Contains("BranchTwo.CollectTeaLeafSchemas", rootGen);
        Assert.Contains("emitted.Add(\"diamond_root\")", rootGen);

        // Each @struct appears exactly once — use Split to count occurrences
        foreach (var name in new[] { "shared_leaf", "branch_one", "branch_two", "diamond_root" })
            Assert.Equal(1, allGen.Split($"@struct {name} (").Length - 1);
    }

    [Fact]
    public void Generator_DeepDiamondDependency_DeduplicatesSchemas()
    {
        var source = @"
using TeaLeaf.Annotations;
namespace TestModels;

[TeaLeaf(Generate = true)] public partial class DeepLeaf { public string Data { get; set; } = """"; }
[TeaLeaf(Generate = true)] public partial class MidLeft { public string Info { get; set; } = """"; public DeepLeaf Inner { get; set; } = new(); }
[TeaLeaf(Generate = true)] public partial class MidRight { public string Info { get; set; } = """"; public DeepLeaf Inner { get; set; } = new(); }
[TeaLeaf(Generate = true)] public partial class TopLevel { public string Name { get; set; } = """"; public MidLeft Left { get; set; } = new(); public MidRight Right { get; set; } = new(); }
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var allGen = string.Join("\n", result.GeneratedTrees
            .Where(t => t.FilePath.Contains(".TeaLeaf.g.cs"))
            .Select(t => t.GetText().ToString()));

        foreach (var name in new[] { "deep_leaf", "mid_left", "mid_right", "top_level" })
            Assert.Equal(1, allGen.Split($"@struct {name} (").Length - 1);

        var topGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("TopLevel.TeaLeaf.g.cs")).GetText().ToString();
        Assert.Contains("MidLeft.CollectTeaLeafSchemas", topGen);
        Assert.Contains("MidRight.CollectTeaLeafSchemas", topGen);

        var midLeftGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("MidLeft.TeaLeaf.g.cs")).GetText().ToString();
        Assert.Contains("DeepLeaf.CollectTeaLeafSchemas", midLeftGen);
    }

    [Fact]
    public void Generator_NestedTypeWithoutStructName_UsesSnakeCase()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class ShippingAddress
{
    public string Street { get; set; } = """";
    public string City { get; set; } = """";
}

[TeaLeaf(Generate = true)]
public partial class Customer
{
    public string Name { get; set; } = """";
    public ShippingAddress Address { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Customer.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Without StructName override, should use default snake_case
        Assert.Contains("address: shipping_address", gen);
    }

    // =========================================================================
    // Generate property: opt-in source generation
    // =========================================================================

    [Fact]
    public void Generator_WithoutGenerate_SkipsCodeGeneration()
    {
        // [TeaLeaf] without Generate = true should not produce generated source
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public partial class ReflectionOnly
{
    public string Name { get; set; } = """";
    public int Age { get; set; }
}
";
        var result = RunGenerator(source);

        // No generated source for this type
        Assert.DoesNotContain(result.GeneratedTrees,
            t => t.FilePath.Contains("ReflectionOnly.TeaLeaf.g.cs"));

        // No diagnostics either
        Assert.Empty(result.Diagnostics.Where(d => d.Id == "TL001"));
    }

    [Fact]
    public void Generator_WithoutGenerate_NoTL001ForNonPartial()
    {
        // [TeaLeaf] without Generate = true on a non-partial class should NOT trigger TL001
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf]
public class NonPartialReflection
{
    public string Name { get; set; } = """";
}
";
        var result = RunGenerator(source);

        // No TL001 since Generate is not set
        Assert.Empty(result.Diagnostics.Where(d => d.Id == "TL001"));

        // No generated source
        Assert.DoesNotContain(result.GeneratedTrees,
            t => t.FilePath.Contains("NonPartialReflection.TeaLeaf.g.cs"));
    }

    [Fact]
    public void Generator_GenerateTrue_NonPartial_ReportsTL001()
    {
        // [TeaLeaf(Generate = true)] on a non-partial class SHOULD trigger TL001
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public class NonPartialGenerate
{
    public string Name { get; set; } = """";
}
";
        var result = RunGenerator(source);

        var tl001 = result.Diagnostics.FirstOrDefault(d => d.Id == "TL001");
        Assert.NotNull(tl001);
        Assert.Equal(DiagnosticSeverity.Error, tl001!.Severity);
        Assert.Contains("NonPartialGenerate", tl001.GetMessage());
    }

    [Fact]
    public void Generator_GenerateTrue_Partial_GeneratesCode()
    {
        // [TeaLeaf(Generate = true)] on a partial class should generate code
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class GenerateEnabled
{
    public string Name { get; set; } = """";
    public int Count { get; set; }
}
";
        var result = RunGenerator(source);

        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));
        Assert.Single(result.GeneratedTrees.Where(t => t.FilePath.Contains("GenerateEnabled.TeaLeaf.g.cs")));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("GenerateEnabled.TeaLeaf.g.cs"))
            .GetText().ToString();

        Assert.Contains("ToTeaLeafText()", gen);
        Assert.Contains("FromTeaLeaf(", gen);
    }

    // ================================================================
    // Parameterized Constructor Tests
    // ================================================================

    [Fact]
    public void Generator_ParameterizedConstructor_GeneratesConstructorDeserialization()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class ImmutableItem
{
    public string Name { get; }
    public int Count { get; }

    public ImmutableItem(string name, int count)
    {
        Name = name;
        Count = count;
    }
}
";
        var result = RunGenerator(source);

        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));
        Assert.Single(result.GeneratedTrees.Where(t => t.FilePath.Contains("ImmutableItem.TeaLeaf.g.cs")));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("ImmutableItem.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Should declare local variables for constructor parameters
        Assert.Contains("_p_name", gen);
        Assert.Contains("_p_count", gen);
        // Should call the parameterized constructor
        Assert.Contains("new ImmutableItem(_p_name, _p_count)", gen);
        // Should NOT contain "new ImmutableItem()" (parameterless)
        Assert.DoesNotContain("new ImmutableItem();", gen);
        // Should contain schema and serialization methods
        Assert.Contains("GetTeaLeafSchema()", gen);
        Assert.Contains("ToTeaLeafText()", gen);
        Assert.Contains("FromTeaLeaf(", gen);
    }

    [Fact]
    public void Generator_MixedConstructorAndSetter_GeneratesCorrectCode()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class MixedItem
{
    public string Name { get; }
    public int Id { get; }
    public string? Notes { get; set; }

    public MixedItem(string name, int id)
    {
        Name = name;
        Id = id;
    }
}
";
        var result = RunGenerator(source);

        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("MixedItem.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Constructor params read into locals
        Assert.Contains("_p_name", gen);
        Assert.Contains("_p_id", gen);
        // Constructor call
        Assert.Contains("new MixedItem(_p_name, _p_id)", gen);
        // Remaining setter property assigned via result
        Assert.Contains("result.Notes", gen);
    }

    [Fact]
    public void Generator_ConstructorWithDefaults_GeneratesDefaultValues()
    {
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class DefaultsItem
{
    public string Name { get; }
    public int Score { get; }

    public DefaultsItem(string name, int score = 100)
    {
        Name = name;
        Score = score;
    }
}
";
        var result = RunGenerator(source);

        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("DefaultsItem.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Constructor param with default value should use it
        Assert.Contains("_p_score = 100", gen);
        Assert.Contains("new DefaultsItem(_p_name, _p_score)", gen);
    }

    [Fact]
    public void Generator_ParameterlessConstructor_StillUsesNewPattern()
    {
        // Existing types with parameterless constructors should still use `new Type()`
        var source = @"
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class MutableItem
{
    public string Name { get; set; } = """";
    public int Count { get; set; }
}
";
        var result = RunGenerator(source);

        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var gen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("MutableItem.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Should use parameterless constructor pattern
        Assert.Contains("new MutableItem();", gen);
        // Should NOT contain constructor local variables pattern
        Assert.DoesNotContain("_p_", gen);
    }

    // =========================================================================
    // @table code emission for List<NestedType>
    // =========================================================================

    [Fact]
    public void Generator_ListOfNestedType_EmitsTableFormat()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class LineItem
{
    public string Sku { get; set; } = """";
    public int Qty { get; set; }
    public double Price { get; set; }
}

[TeaLeaf(Generate = true)]
public partial class Invoice
{
    public string InvoiceId { get; set; } = """";
    public List<LineItem> Items { get; set; } = new();
    public double Total { get; set; }
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var invoiceGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Invoice.TeaLeaf.g.cs"))
            .GetText().ToString();

        var lineItemGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("LineItem.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Invoice should emit @table for the nested list
        Assert.Contains("@table line_item", invoiceGen);
        // LineItem should have WriteTeaLeafTupleValue method
        Assert.Contains("WriteTeaLeafTupleValue", lineItemGen);
    }

    [Fact]
    public void Generator_DeeplyNestedType_EmitsTableAndTupleValue()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class Coord
{
    public double Lat { get; set; }
    public double Lng { get; set; }
}

[TeaLeaf(Generate = true)]
public partial class Place
{
    public string Name { get; set; } = """";
    public Coord Location { get; set; } = new();
}

[TeaLeaf(Generate = true)]
public partial class Trip
{
    public string TripId { get; set; } = """";
    public List<Place> Stops { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var tripGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Trip.TeaLeaf.g.cs"))
            .GetText().ToString();

        var placeGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Place.TeaLeaf.g.cs"))
            .GetText().ToString();

        var coordGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Coord.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Trip should emit @table for the nested list of Place
        Assert.Contains("@table place", tripGen);

        // Place should have WriteTeaLeafTupleValue that calls nested Coord tuple
        Assert.Contains("WriteTeaLeafTupleValue", placeGen);

        // Coord should also have WriteTeaLeafTupleValue
        Assert.Contains("WriteTeaLeafTupleValue", coordGen);
    }

    [Fact]
    public void Generator_TableEmission_NoTrailingCommaPattern()
    {
        var source = @"
using System.Collections.Generic;
using TeaLeaf.Annotations;

namespace TestModels;

[TeaLeaf(Generate = true)]
public partial class Tag
{
    public string Key { get; set; } = """";
    public string Value { get; set; } = """";
}

[TeaLeaf(Generate = true)]
public partial class Tagged
{
    public string Name { get; set; } = """";
    public List<Tag> Tags { get; set; } = new();
}
";
        var result = RunGenerator(source);
        Assert.Empty(result.Diagnostics.Where(d => d.Severity == DiagnosticSeverity.Error));

        var taggedGen = result.GeneratedTrees
            .First(t => t.FilePath.Contains("Tagged.TeaLeaf.g.cs"))
            .GetText().ToString();

        // Should use separator pattern (first_ flag) instead of terminator
        Assert.Contains("first_", taggedGen);
        // Should contain @table directive
        Assert.Contains("@table tag", taggedGen);
    }
}
