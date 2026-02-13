using TeaLeaf.Annotations;
using Xunit;

namespace TeaLeaf.Tests;

public class TLDocumentBuilderTests
{
    // ------------------------------------------------------------------
    // Single key
    // ------------------------------------------------------------------

    [Fact]
    public void Build_SingleKey_ProducesValidDocument()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        using var doc = new TLDocumentBuilder()
            .Add("user", user)
            .Build();

        Assert.NotNull(doc);
        Assert.Contains("user", doc.Keys);

        using var value = doc["user"];
        Assert.NotNull(value);
        Assert.Equal(TLType.Object, value.Type);

        using var name = value["name"];
        Assert.Equal("alice", name?.AsString());

        using var age = value["age"];
        Assert.Equal(30, age?.AsInt());

        using var active = value["active"];
        Assert.True(active?.AsBool());
    }

    // ------------------------------------------------------------------
    // Multiple keys
    // ------------------------------------------------------------------

    [Fact]
    public void Build_MultipleKeys_PreservesAll()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };
        var config = new AppConfig { AppName = "TestApp", MaxRetries = 3, DebugMode = true };

        using var doc = new TLDocumentBuilder()
            .Add("user", user)
            .Add("config", config)
            .Build();

        Assert.Contains("user", doc.Keys);
        Assert.Contains("config", doc.Keys);

        using var userVal = doc["user"];
        Assert.NotNull(userVal);
        using var name = userVal["name"];
        Assert.Equal("alice", name?.AsString());

        using var configVal = doc["config"];
        Assert.NotNull(configVal);
        using var appName = configVal["app_name"];
        Assert.Equal("TestApp", appName?.AsString());
    }

    // ------------------------------------------------------------------
    // Schemas included
    // ------------------------------------------------------------------

    [Fact]
    public void Build_WithSchemas_IncludesStructDefinitions()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };

        using var doc = new TLDocumentBuilder()
            .Add("user", user)
            .Build();

        // The document text should include @struct definition
        var text = doc.ToText();
        Assert.Contains("@struct simple_user", text);
    }

    // ------------------------------------------------------------------
    // List support
    // ------------------------------------------------------------------

    [Fact]
    public void Build_AddList_ProducesValidDocument()
    {
        var users = new List<SimpleUser>
        {
            new SimpleUser { Name = "alice", Age = 30, Active = true },
            new SimpleUser { Name = "bob", Age = 25, Active = false }
        };

        using var doc = new TLDocumentBuilder()
            .AddList("users", users)
            .Build();

        using var value = doc["users"];
        Assert.NotNull(value);
        Assert.Equal(TLType.Array, value.Type);
        Assert.Equal(2, value.ArrayLength);

        using var first = value[0];
        Assert.NotNull(first);
        using var name = first["name"];
        Assert.Equal("alice", name?.AsString());
    }

    // ------------------------------------------------------------------
    // Nested types
    // ------------------------------------------------------------------

    [Fact]
    public void Build_NestedTypes_IncludesAllSchemas()
    {
        var person = new PersonWithAddress
        {
            Name = "alice",
            HomeAddress = new Address { Street = "123 Main St", City = "Springfield", Zip = "62701" }
        };

        using var doc = new TLDocumentBuilder()
            .Add("person", person)
            .Build();

        var text = doc.ToText();
        Assert.Contains("@struct address", text);
        Assert.Contains("@struct person_with_address", text);

        using var value = doc["person"];
        Assert.NotNull(value);
        using var nameVal = value["name"];
        Assert.Equal("alice", nameVal?.AsString());
    }

    // ------------------------------------------------------------------
    // AddDocument merge
    // ------------------------------------------------------------------

    [Fact]
    public void Build_AddDocument_MergesContent()
    {
        using var existingDoc = TLDocument.Parse("greeting: hello\ncount: 42");

        using var doc = new TLDocumentBuilder()
            .AddDocument(existingDoc)
            .Build();

        using var greeting = doc["greeting"];
        Assert.Equal("hello", greeting?.AsString());

        using var count = doc["count"];
        Assert.Equal(42, count?.AsInt());
    }

    // ------------------------------------------------------------------
    // Mixed Add + AddDocument
    // ------------------------------------------------------------------

    [Fact]
    public void Build_MixedAddAndAddDocument_ProducesValidDocument()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };
        using var rawDoc = TLDocument.Parse("version: 2");

        using var doc = new TLDocumentBuilder()
            .Add("user", user)
            .AddDocument(rawDoc)
            .Build();

        Assert.Contains("user", doc.Keys);
        Assert.Contains("version", doc.Keys);

        using var userVal = doc["user"];
        Assert.NotNull(userVal);

        using var version = doc["version"];
        Assert.Equal(2, version?.AsInt());
    }

    // ------------------------------------------------------------------
    // JSON round-trip
    // ------------------------------------------------------------------

    [Fact]
    public void Build_ToJson_ContainsAllKeys()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };
        var config = new AppConfig { AppName = "MyApp", MaxRetries = 5, DebugMode = false };

        using var doc = new TLDocumentBuilder()
            .Add("user", user)
            .Add("config", config)
            .Build();

        var json = doc.ToJson();
        Assert.Contains("alice", json);
        Assert.Contains("MyApp", json);
    }

    // ------------------------------------------------------------------
    // Binary compile round-trip
    // ------------------------------------------------------------------

    [Fact]
    public void Build_Compile_RoundTrips()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };

        var path = Path.Combine(Path.GetTempPath(), $"tealeaf_builder_test_{Guid.NewGuid()}.tlbx");
        try
        {
            using var doc = new TLDocumentBuilder()
                .Add("user", user)
                .Build();

            doc.Compile(path);

            using var reader = TLReader.Open(path);
            var json = reader.ToJson();
            Assert.Contains("alice", json);
            Assert.Contains("30", json);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    // ------------------------------------------------------------------
    // Chaining
    // ------------------------------------------------------------------

    [Fact]
    public void Builder_MethodsReturnThis_ForChaining()
    {
        var builder = new TLDocumentBuilder();
        var user = new SimpleUser { Name = "test", Age = 1, Active = true };

        var result = builder.Add("user", user);
        Assert.Same(builder, result);

        var result2 = builder.AddList("users", new[] { user });
        Assert.Same(builder, result2);

        using var rawDoc = TLDocument.Parse("key: value");
        var result3 = builder.AddDocument(rawDoc);
        Assert.Same(builder, result3);
    }

    // ------------------------------------------------------------------
    // Scalar overloads
    // ------------------------------------------------------------------

    [Fact]
    public void Add_String_ProducesValidDocument()
    {
        using var doc = new TLDocumentBuilder()
            .Add("greeting", "Hello, TeaLeaf!")
            .Build();

        using var val = doc["greeting"];
        Assert.NotNull(val);
        Assert.Equal(TLType.String, val.Type);
        Assert.Equal("Hello, TeaLeaf!", val.AsString());
    }

    [Fact]
    public void Add_Int_ProducesValidDocument()
    {
        using var doc = new TLDocumentBuilder()
            .Add("count", 42)
            .Build();

        using var val = doc["count"];
        Assert.NotNull(val);
        Assert.Equal(42, val.AsInt());
    }

    [Fact]
    public void Add_Long_ProducesValidDocument()
    {
        using var doc = new TLDocumentBuilder()
            .Add("big", 9_000_000_000L)
            .Build();

        using var val = doc["big"];
        Assert.NotNull(val);
        Assert.Equal(9_000_000_000L, val.AsInt());
    }

    [Fact]
    public void Add_Double_ProducesValidDocument()
    {
        using var doc = new TLDocumentBuilder()
            .Add("pi", 3.14159)
            .Build();

        using var val = doc["pi"];
        Assert.NotNull(val);
        Assert.True(Math.Abs(3.14159 - val.AsFloat()!.Value) < 0.0001);
    }

    [Fact]
    public void Add_Bool_ProducesValidDocument()
    {
        using var doc = new TLDocumentBuilder()
            .Add("enabled", true)
            .Add("disabled", false)
            .Build();

        using var enabled = doc["enabled"];
        Assert.True(enabled?.AsBool());

        using var disabled = doc["disabled"];
        Assert.False(disabled?.AsBool());
    }

    [Fact]
    public void Add_DateTimeOffset_ProducesValidDocument()
    {
        var ts = new DateTimeOffset(2025, 6, 15, 12, 0, 0, TimeSpan.Zero);
        using var doc = new TLDocumentBuilder()
            .Add("created", ts)
            .Build();

        using var val = doc["created"];
        Assert.NotNull(val);
        var parsed = val.AsDateTime();
        Assert.NotNull(parsed);
        Assert.Equal(2025, parsed.Value.Year);
        Assert.Equal(6, parsed.Value.Month);
    }

    [Fact]
    public void Add_MixedScalarsAndObjects_ProducesValidDocument()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };

        using var doc = new TLDocumentBuilder()
            .Add("title", "User Report")
            .Add("version", 2)
            .Add("user", user)
            .Add("debug", false)
            .Build();

        using var title = doc["title"];
        Assert.Equal("User Report", title?.AsString());

        using var version = doc["version"];
        Assert.Equal(2, version?.AsInt());

        using var userVal = doc["user"];
        Assert.Equal(TLType.Object, userVal?.Type);

        using var debug = doc["debug"];
        Assert.False(debug?.AsBool());
    }

    // ------------------------------------------------------------------
    // Scalar list overloads
    // ------------------------------------------------------------------

    [Fact]
    public void AddList_Strings_ProducesValidArray()
    {
        using var doc = new TLDocumentBuilder()
            .AddList("tags", new[] { "alpha", "beta", "gamma" })
            .Build();

        using var val = doc["tags"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Array, val.Type);
        Assert.Equal(3, val.ArrayLength);

        using var first = val[0];
        Assert.Equal("alpha", first?.AsString());
    }

    [Fact]
    public void AddList_Ints_ProducesValidArray()
    {
        using var doc = new TLDocumentBuilder()
            .AddList("scores", new[] { 100, 200, 300 })
            .Build();

        using var val = doc["scores"];
        Assert.NotNull(val);
        Assert.Equal(3, val.ArrayLength);

        using var second = val[1];
        Assert.Equal(200, second?.AsInt());
    }

    [Fact]
    public void AddList_Doubles_ProducesValidArray()
    {
        using var doc = new TLDocumentBuilder()
            .AddList("weights", new[] { 1.5, 2.5, 3.5 })
            .Build();

        using var val = doc["weights"];
        Assert.NotNull(val);
        Assert.Equal(3, val.ArrayLength);

        using var third = val[2];
        Assert.True(Math.Abs(3.5 - third!.AsFloat()!.Value) < 0.001);
    }
}
