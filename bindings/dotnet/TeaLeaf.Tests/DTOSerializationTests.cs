using TeaLeaf.Annotations;
using Xunit;

namespace TeaLeaf.Tests;

// ========================================================================
// Test DTO Models
// ========================================================================

[TeaLeaf(Generate = true)]
public partial class SimpleUser
{
    public string Name { get; set; } = "";
    public int Age { get; set; }
    public bool Active { get; set; }
}

[TeaLeaf(Generate = true, StructName = "custom_product")]
public partial class Product
{
    [TLRename("product_name")]
    public string Name { get; set; } = "";

    public double Price { get; set; }

    [TLSkip]
    public string InternalSku { get; set; } = "";

    public string? Description { get; set; }
}

public enum UserRole { Admin, Editor, Viewer }

[TeaLeaf(Generate = true)]
public partial class UserWithEnum
{
    public string Name { get; set; } = "";
    public UserRole Role { get; set; }
}

[TeaLeaf(Generate = true)]
public partial class Address
{
    public string Street { get; set; } = "";
    public string City { get; set; } = "";
    public string Zip { get; set; } = "";
}

[TeaLeaf(Generate = true)]
public partial class PersonWithAddress
{
    public string Name { get; set; } = "";
    public Address HomeAddress { get; set; } = new();
}

[TeaLeaf(Generate = true)]
public partial class WithCollections
{
    public string Name { get; set; } = "";
    public List<string> Tags { get; set; } = new();
    public List<int> Scores { get; set; } = new();
}

[TeaLeaf(Generate = true)]
[TLKey("my_config")]
public partial class AppConfig
{
    public string AppName { get; set; } = "";
    public int MaxRetries { get; set; }
    public bool DebugMode { get; set; }
}

[TeaLeaf(Generate = true)]
public partial class NullableFields
{
    public string Name { get; set; } = "";
    public string? Email { get; set; }
    public int? Age { get; set; }

    [TLOptional]
    public int Score { get; set; }
}

// ========================================================================
// Test DTOs for Nested Struct Tests
// ========================================================================

[TeaLeaf(Generate = true)]
public partial class TestOrderItem
{
    [TLRename("product_name")]
    public string ProductName { get; set; } = "";
    public int Quantity { get; set; }
    [TLRename("unit_price")]
    public double UnitPrice { get; set; }
}

[TeaLeaf(Generate = true)]
[TLKey("test_order")]
public partial class TestOrderWithItems
{
    [TLRename("order_id")]
    public string OrderId { get; set; } = "";
    public List<TestOrderItem> Items { get; set; } = new();
    public double Total { get; set; }
}

// ========================================================================
// Tests
// ========================================================================

public class DTOSerializationTests
{
    // ------------------------------------------------------------------
    // Schema Generation
    // ------------------------------------------------------------------

    [Fact]
    public void GetSchema_SimpleUser_ContainsFieldDefinitions()
    {
        var schema = SimpleUser.GetTeaLeafSchema();

        Assert.Contains("@struct simple_user", schema);
        Assert.Contains("name: string", schema);
        Assert.Contains("age: int", schema);
        Assert.Contains("active: bool", schema);
    }

    [Fact]
    public void GetSchema_Product_UsesCustomStructName()
    {
        var schema = Product.GetTeaLeafSchema();

        Assert.Contains("@struct custom_product", schema);
        Assert.Contains("product_name: string", schema);
        Assert.Contains("price: float", schema);
        // Skipped field should not appear
        Assert.DoesNotContain("internal_sku", schema);
    }

    [Fact]
    public void GetSchema_NullableFields_MarkedWithQuestionMark()
    {
        var schema = NullableFields.GetTeaLeafSchema();

        Assert.Contains("email: string?", schema);
        Assert.Contains("age: int?", schema);
        Assert.Contains("score: int?", schema);
    }

    // ------------------------------------------------------------------
    // Text Serialization
    // ------------------------------------------------------------------

    [Fact]
    public void ToTeaLeafText_SimpleUser_ContainsFieldValues()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        var text = user.ToTeaLeafText();

        Assert.Contains("name:", text);
        Assert.Contains("alice", text);
        Assert.Contains("age:", text);
        Assert.Contains("30", text);
        Assert.Contains("active:", text);
        Assert.Contains("true", text);
    }

    [Fact]
    public void ToTeaLeafText_Product_UsesRenamedFields()
    {
        var product = new Product
        {
            Name = "Widget",
            Price = 19.99,
            InternalSku = "SKU-123",
            Description = "A fine widget"
        };

        var text = product.ToTeaLeafText();

        Assert.Contains("product_name:", text);
        Assert.Contains("Widget", text);
        Assert.Contains("price:", text);
        Assert.Contains("19.99", text);
        // Skipped field should not appear
        Assert.DoesNotContain("SKU-123", text);
        Assert.DoesNotContain("internal_sku", text);
    }

    [Fact]
    public void ToTeaLeafText_BoolValues_SerializeAsTrueFalse()
    {
        var activeUser = new SimpleUser { Name = "bob", Age = 25, Active = true };
        var inactiveUser = new SimpleUser { Name = "carol", Age = 35, Active = false };

        Assert.Contains("true", activeUser.ToTeaLeafText());
        Assert.Contains("false", inactiveUser.ToTeaLeafText());
    }

    [Fact]
    public void ToTeaLeafText_NullableNull_SerializesAsTilde()
    {
        var model = new NullableFields
        {
            Name = "test",
            Email = null,
            Age = null
        };

        var text = model.ToTeaLeafText();

        Assert.Contains("email: ~", text);
    }

    // ------------------------------------------------------------------
    // Document Creation (requires native library)
    // ------------------------------------------------------------------

    [Fact]
    public void ToTeaLeafDocument_SimpleUser_ProducesValidTLText()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        var docText = user.ToTeaLeafDocument();

        Assert.Contains("@struct simple_user", docText);
        Assert.Contains("simple_user:", docText);
        Assert.Contains("name:", docText);
        Assert.Contains("alice", docText);
    }

    [Fact]
    public void ToTLDocument_SimpleUser_ParsesCorrectly()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        using var doc = user.ToTLDocument();
        Assert.NotNull(doc);

        using var value = doc["simple_user"];
        Assert.NotNull(value);
        Assert.Equal(TLType.Object, value.Type);

        using var name = value["name"];
        Assert.Equal("alice", name?.AsString());

        using var age = value["age"];
        Assert.Equal(30, age?.AsInt());

        using var active = value["active"];
        Assert.True(active?.AsBool());
    }

    [Fact]
    public void ToTLDocument_Product_UsesCustomNames()
    {
        var product = new Product
        {
            Name = "Gadget",
            Price = 29.99,
            InternalSku = "SKIP-ME"
        };

        using var doc = product.ToTLDocument();
        using var value = doc["custom_product"];
        Assert.NotNull(value);

        using var name = value["product_name"];
        Assert.Equal("Gadget", name?.AsString());

        using var price = value["price"];
        var priceVal = price?.AsFloat();
        Assert.NotNull(priceVal);
        Assert.True(Math.Abs(29.99 - priceVal.Value) < 0.01);

        // Skipped field should not be present
        using var sku = value["internal_sku"];
        Assert.Null(sku);
    }

    [Fact]
    public void ToTLDocument_WithCustomKey_UsesCorrectKey()
    {
        var config = new AppConfig
        {
            AppName = "MyApp",
            MaxRetries = 3,
            DebugMode = true
        };

        using var doc = config.ToTLDocument();

        // Should use the custom key from [TLKey]
        using var value = doc["my_config"];
        Assert.NotNull(value);

        using var appName = value["app_name"];
        Assert.Equal("MyApp", appName?.AsString());
    }

    // ------------------------------------------------------------------
    // JSON Serialization (requires native library)
    // ------------------------------------------------------------------

    [Fact]
    public void ToTeaLeafJson_SimpleUser_ProducesValidJson()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        var json = user.ToTeaLeafJson();

        Assert.NotNull(json);
        Assert.Contains("alice", json);
        Assert.Contains("30", json);
    }

    // ------------------------------------------------------------------
    // Deserialization (requires native library)
    // ------------------------------------------------------------------

    [Fact]
    public void FromTeaLeaf_SimpleUser_DeserializesCorrectly()
    {
        using var doc = TLDocument.Parse(@"
            @struct simple_user (name: string, age: int, active: bool)

            simple_user: {
                name: alice
                age: 30
                active: true
            }
        ");

        var user = SimpleUser.FromTeaLeaf(doc);

        Assert.Equal("alice", user.Name);
        Assert.Equal(30, user.Age);
        Assert.True(user.Active);
    }

    [Fact]
    public void FromTeaLeaf_Product_HandlesRename()
    {
        using var doc = TLDocument.Parse(@"
            @struct custom_product (product_name: string, price: float)

            custom_product: {
                product_name: Widget
                price: 19.99
            }
        ");

        var product = Product.FromTeaLeaf(doc);

        Assert.Equal("Widget", product.Name);
        Assert.True(Math.Abs(19.99 - product.Price) < 0.01);
        // Skipped field should remain default
        Assert.Equal("", product.InternalSku);
    }

    // ------------------------------------------------------------------
    // Round-Trip Tests (serialize then deserialize)
    // ------------------------------------------------------------------

    [Fact]
    public void RoundTrip_SimpleUser_PreservesData()
    {
        var original = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        using var doc = original.ToTLDocument();
        var restored = SimpleUser.FromTeaLeaf(doc);

        Assert.Equal(original.Name, restored.Name);
        Assert.Equal(original.Age, restored.Age);
        Assert.Equal(original.Active, restored.Active);
    }

    [Fact]
    public void RoundTrip_Product_PreservesDataWithRenames()
    {
        var original = new Product
        {
            Name = "Fancy Widget",
            Price = 49.99,
            InternalSku = "not-serialized",
            Description = "A very fancy widget"
        };

        using var doc = original.ToTLDocument();
        var restored = Product.FromTeaLeaf(doc);

        Assert.Equal(original.Name, restored.Name);
        Assert.True(Math.Abs(original.Price - restored.Price) < 0.01);
        // Skipped field is lost through roundtrip (expected)
        Assert.Equal("", restored.InternalSku);
    }

    [Fact]
    public void RoundTrip_AppConfig_UsesCustomKey()
    {
        var original = new AppConfig
        {
            AppName = "TestApp",
            MaxRetries = 5,
            DebugMode = false
        };

        using var doc = original.ToTLDocument();
        var restored = AppConfig.FromTeaLeaf(doc);

        Assert.Equal(original.AppName, restored.AppName);
        Assert.Equal(original.MaxRetries, restored.MaxRetries);
        Assert.Equal(original.DebugMode, restored.DebugMode);
    }

    [Fact]
    public void RoundTrip_WithCollections_PreservesLists()
    {
        var original = new WithCollections
        {
            Name = "test",
            Tags = new List<string> { "alpha", "beta", "gamma" },
            Scores = new List<int> { 10, 20, 30 }
        };

        using var doc = original.ToTLDocument();
        var restored = WithCollections.FromTeaLeaf(doc);

        Assert.Equal(original.Name, restored.Name);
        Assert.Equal(original.Tags, restored.Tags);
        Assert.Equal(original.Scores, restored.Scores);
    }

    // ------------------------------------------------------------------
    // Text Round-Trip (serialize to text, parse back, verify)
    // ------------------------------------------------------------------

    [Fact]
    public void TextRoundTrip_SimpleUser_ProducesValidTL()
    {
        var user = new SimpleUser
        {
            Name = "bob",
            Age = 25,
            Active = false
        };

        var tlText = user.ToTeaLeafDocument();

        // Should be parseable
        using var doc = TLDocument.Parse(tlText);
        Assert.NotNull(doc);

        using var value = doc["simple_user"];
        Assert.NotNull(value);

        using var name = value["name"];
        Assert.Equal("bob", name?.AsString());
    }

    // ------------------------------------------------------------------
    // Edge Cases
    // ------------------------------------------------------------------

    [Fact]
    public void Serialization_SpecialCharsInString_QuotedCorrectly()
    {
        var user = new SimpleUser
        {
            Name = "alice \"admin\" jones",
            Age = 30,
            Active = true
        };

        var text = user.ToTeaLeafText();

        // String with special chars should be quoted
        Assert.Contains("\"", text);
    }

    [Fact]
    public void RoundTrip_EmptyStrings_Preserved()
    {
        var user = new SimpleUser
        {
            Name = "",
            Age = 0,
            Active = false
        };

        using var doc = user.ToTLDocument();
        var restored = SimpleUser.FromTeaLeaf(doc);

        Assert.Equal("", restored.Name);
        Assert.Equal(0, restored.Age);
        Assert.False(restored.Active);
    }

    [Fact]
    public void RoundTrip_LargeNumbers_Preserved()
    {
        var user = new SimpleUser
        {
            Name = "test",
            Age = 2_000_000_000,
            Active = true
        };

        using var doc = user.ToTLDocument();
        var restored = SimpleUser.FromTeaLeaf(doc);

        Assert.Equal(2_000_000_000, restored.Age);
    }

    // ------------------------------------------------------------------
    // Regression: Digit-prefix string quoting
    // ------------------------------------------------------------------

    [Fact]
    public void RoundTrip_DigitPrefixString_PreservesData()
    {
        var product = new Product
        {
            Name = "44mm",
            Price = 19.99
        };

        using var doc = product.ToTLDocument();
        var restored = Product.FromTeaLeaf(doc);

        Assert.Equal("44mm", restored.Name);
    }

    [Fact]
    public void Serialization_DigitPrefixString_IsQuoted()
    {
        var user = new SimpleUser { Name = "44mm", Age = 1, Active = true };
        var text = user.ToTeaLeafText();
        // "44mm" must be quoted to prevent parser from splitting into 44 + mm
        Assert.Contains("\"44mm\"", text);
    }

    // ------------------------------------------------------------------
    // Regression: Slash character quoting
    // ------------------------------------------------------------------

    [Fact]
    public void RoundTrip_SlashInString_PreservesData()
    {
        var product = new Product
        {
            Name = "Electronics/Audio",
            Price = 9.99
        };

        using var doc = product.ToTLDocument();
        var restored = Product.FromTeaLeaf(doc);

        Assert.Equal("Electronics/Audio", restored.Name);
    }

    [Fact]
    public void Serialization_SlashInString_IsQuoted()
    {
        var user = new SimpleUser { Name = "a/b", Age = 1, Active = true };
        var text = user.ToTeaLeafText();
        Assert.Contains("\"a/b\"", text);
    }

    // ------------------------------------------------------------------
    // Regression: Int-to-float coercion (whole-number doubles)
    // ------------------------------------------------------------------

    [Fact]
    public void RoundTrip_WholeNumberDouble_PreservesValue()
    {
        var product = new Product
        {
            Name = "test",
            Price = 1423.0
        };

        using var doc = product.ToTLDocument();
        var restored = Product.FromTeaLeaf(doc);

        Assert.Equal(1423.0, restored.Price, 5);
    }

    [Fact]
    public void RoundTrip_ZeroDouble_PreservesValue()
    {
        var product = new Product
        {
            Name = "free",
            Price = 0.0
        };

        using var doc = product.ToTLDocument();
        var restored = Product.FromTeaLeaf(doc);

        Assert.Equal(0.0, restored.Price, 5);
    }

    // ------------------------------------------------------------------
    // Nested struct schema and round-trip tests
    // ------------------------------------------------------------------

    [Fact]
    public void Schema_NestedObject_IncludesNestedStructDefinition()
    {
        var doc = new PersonWithAddress
        {
            Name = "test",
            HomeAddress = new Address { Street = "x", City = "y", Zip = "z" }
        }.ToTeaLeafDocument();

        Assert.Contains("@struct address", doc);
        Assert.Contains("@struct person_with_address", doc);
        Assert.Contains("home_address: address", doc);
    }

    [Fact]
    public void Schema_ListOfNestedObjects_IncludesAllStructDefinitions()
    {
        var order = new TestOrderWithItems
        {
            OrderId = "test",
            Items = new List<TestOrderItem>
            {
                new TestOrderItem { ProductName = "x", Quantity = 1, UnitPrice = 1.0 }
            },
            Total = 1.0
        };

        var doc = order.ToTeaLeafDocument();

        Assert.Contains("@struct test_order_item", doc);
        Assert.Contains("@struct test_order_with_items", doc);
    }

    [Fact]
    public void RoundTrip_NestedObject_PreservesAllFields()
    {
        var original = new PersonWithAddress
        {
            Name = "alice",
            HomeAddress = new Address
            {
                Street = "123 Main St",
                City = "Springfield",
                Zip = "62701"
            }
        };

        using var doc = original.ToTLDocument();
        var restored = PersonWithAddress.FromTeaLeaf(doc);

        Assert.Equal("alice", restored.Name);
        Assert.Equal("123 Main St", restored.HomeAddress.Street);
        Assert.Equal("Springfield", restored.HomeAddress.City);
        Assert.Equal("62701", restored.HomeAddress.Zip);
    }

    [Fact]
    public void RoundTrip_ListOfNestedObjects_PreservesData()
    {
        var original = new TestOrderWithItems
        {
            OrderId = "ORD-001",
            Items = new List<TestOrderItem>
            {
                new TestOrderItem { ProductName = "Widget", Quantity = 2, UnitPrice = 9.99 },
                new TestOrderItem { ProductName = "Gadget", Quantity = 1, UnitPrice = 24.99 }
            },
            Total = 44.97
        };

        using var doc = original.ToTLDocument();
        var restored = TestOrderWithItems.FromTeaLeaf(doc);

        Assert.Equal("ORD-001", restored.OrderId);
        Assert.Equal(2, restored.Items.Count);
        Assert.Equal("Widget", restored.Items[0].ProductName);
        Assert.Equal(2, restored.Items[0].Quantity);
        Assert.True(Math.Abs(9.99 - restored.Items[0].UnitPrice) < 0.01);
        Assert.Equal("Gadget", restored.Items[1].ProductName);
        Assert.Equal(1, restored.Items[1].Quantity);
        Assert.True(Math.Abs(44.97 - restored.Total) < 0.01);
    }

    [Fact]
    public void BinaryCompile_NestedObject_RoundTrips()
    {
        var original = new PersonWithAddress
        {
            Name = "bob",
            HomeAddress = new Address
            {
                Street = "456 Oak Ave",
                City = "Portland",
                Zip = "97201"
            }
        };

        var path = Path.Combine(Path.GetTempPath(), $"tealeaf_test_nested_{Guid.NewGuid()}.tlbx");
        try
        {
            original.CompileToTeaLeaf(path);

            using var reader = TLReader.Open(path);
            var json = reader.ToJson();
            Assert.Contains("bob", json);
            Assert.Contains("456 Oak Ave", json);
            Assert.Contains("Portland", json);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void BinaryCompile_ListOfNestedObjects_RoundTrips()
    {
        var original = new TestOrderWithItems
        {
            OrderId = "ORD-002",
            Items = new List<TestOrderItem>
            {
                new TestOrderItem { ProductName = "Alpha", Quantity = 3, UnitPrice = 15.0 },
                new TestOrderItem { ProductName = "Beta", Quantity = 1, UnitPrice = 42.0 }
            },
            Total = 87.0
        };

        var path = Path.Combine(Path.GetTempPath(), $"tealeaf_test_list_{Guid.NewGuid()}.tlbx");
        try
        {
            original.CompileToTeaLeaf(path);

            using var reader = TLReader.Open(path);
            var json = reader.ToJson();
            Assert.Contains("ORD-002", json);
            Assert.Contains("Alpha", json);
            Assert.Contains("Beta", json);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }
}

// ========================================================================
// TeaLeafSerializer Tests (reflection-based)
// ========================================================================

public class TeaLeafSerializerTests
{
    // ------------------------------------------------------------------
    // Schema Generation
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_GetSchema_SimpleUser()
    {
        var schema = TeaLeafSerializer.GetSchema<SimpleUser>();

        Assert.Contains("@struct simple_user", schema);
        Assert.Contains("name: string", schema);
        Assert.Contains("age: int", schema);
        Assert.Contains("active: bool", schema);
    }

    [Fact]
    public void Serializer_GetSchema_Product_UsesCustomStructName()
    {
        var schema = TeaLeafSerializer.GetSchema<Product>();

        Assert.Contains("@struct custom_product", schema);
        Assert.Contains("product_name: string", schema);
        Assert.Contains("price: float", schema);
        Assert.DoesNotContain("internal_sku", schema);
    }

    // ------------------------------------------------------------------
    // Text Serialization
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_ToText_SimpleUser()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        var text = TeaLeafSerializer.ToText(user);

        Assert.Contains("name:", text);
        Assert.Contains("alice", text);
        Assert.Contains("age:", text);
        Assert.Contains("30", text);
        Assert.Contains("active:", text);
        Assert.Contains("true", text);
    }

    [Fact]
    public void Serializer_ToText_Product_SkipsAndRenames()
    {
        var product = new Product
        {
            Name = "Widget",
            Price = 19.99,
            InternalSku = "SKU-123",
            Description = "A fine widget"
        };

        var text = TeaLeafSerializer.ToText(product);

        Assert.Contains("product_name:", text);
        Assert.Contains("Widget", text);
        Assert.Contains("price:", text);
        Assert.DoesNotContain("SKU-123", text);
        Assert.DoesNotContain("internal_sku", text);
    }

    // ------------------------------------------------------------------
    // Document Creation
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_ToDocument_SimpleUser_ProducesValidTL()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        var docText = TeaLeafSerializer.ToDocument(user);

        Assert.Contains("@struct simple_user", docText);
        Assert.Contains("simple_user:", docText);
        Assert.Contains("name:", docText);
        Assert.Contains("alice", docText);
    }

    [Fact]
    public void Serializer_ToTLDocument_SimpleUser_ParsesCorrectly()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        using var doc = TeaLeafSerializer.ToTLDocument(user);
        Assert.NotNull(doc);

        using var value = doc["simple_user"];
        Assert.NotNull(value);

        using var name = value["name"];
        Assert.Equal("alice", name?.AsString());

        using var age = value["age"];
        Assert.Equal(30, age?.AsInt());
    }

    // ------------------------------------------------------------------
    // Deserialization
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_FromText_SimpleUser()
    {
        var tlText = @"
            @struct simple_user (name: string, age: int, active: bool)

            simple_user: {
                name: alice
                age: 30
                active: true
            }
        ";

        var user = TeaLeafSerializer.FromText<SimpleUser>(tlText);

        Assert.Equal("alice", user.Name);
        Assert.Equal(30, user.Age);
        Assert.True(user.Active);
    }

    [Fact]
    public void Serializer_FromDocument_Product_HandlesRename()
    {
        using var doc = TLDocument.Parse(@"
            @struct custom_product (product_name: string, price: float)

            custom_product: {
                product_name: Widget
                price: 19.99
            }
        ");

        var product = TeaLeafSerializer.FromDocument<Product>(doc);

        Assert.Equal("Widget", product.Name);
        Assert.True(Math.Abs(19.99 - product.Price) < 0.01);
        Assert.Equal("", product.InternalSku);
    }

    // ------------------------------------------------------------------
    // Round-Trip
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_RoundTrip_SimpleUser()
    {
        var original = new SimpleUser
        {
            Name = "bob",
            Age = 25,
            Active = false
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<SimpleUser>(doc);

        Assert.Equal(original.Name, restored.Name);
        Assert.Equal(original.Age, restored.Age);
        Assert.Equal(original.Active, restored.Active);
    }

    [Fact]
    public void Serializer_RoundTrip_Product()
    {
        var original = new Product
        {
            Name = "Fancy Widget",
            Price = 49.99,
            InternalSku = "not-serialized",
            Description = "A very fancy widget"
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<Product>(doc);

        Assert.Equal(original.Name, restored.Name);
        Assert.True(Math.Abs(original.Price - restored.Price) < 0.01);
        Assert.Equal("", restored.InternalSku); // skipped, lost in round-trip
    }

    [Fact]
    public void Serializer_RoundTrip_AppConfig_UsesCustomKey()
    {
        var original = new AppConfig
        {
            AppName = "TestApp",
            MaxRetries = 5,
            DebugMode = false
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<AppConfig>(doc);

        Assert.Equal(original.AppName, restored.AppName);
        Assert.Equal(original.MaxRetries, restored.MaxRetries);
        Assert.Equal(original.DebugMode, restored.DebugMode);
    }

    [Fact]
    public void Serializer_RoundTrip_WithCollections()
    {
        var original = new WithCollections
        {
            Name = "test",
            Tags = new List<string> { "alpha", "beta", "gamma" },
            Scores = new List<int> { 10, 20, 30 }
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<WithCollections>(doc);

        Assert.Equal(original.Name, restored.Name);
        Assert.Equal(original.Tags, restored.Tags);
        Assert.Equal(original.Scores, restored.Scores);
    }

    // ------------------------------------------------------------------
    // Consistency: Serializer vs Source-Generated
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_SimpleUser_SchemaMatchesGenerated()
    {
        var generatedSchema = SimpleUser.GetTeaLeafSchema();
        var reflectionSchema = TeaLeafSerializer.GetSchema<SimpleUser>();

        // Both should contain the same struct definition
        Assert.Contains("@struct simple_user", generatedSchema);
        Assert.Contains("@struct simple_user", reflectionSchema);
        Assert.Contains("name: string", reflectionSchema);
        Assert.Contains("age: int", reflectionSchema);
        Assert.Contains("active: bool", reflectionSchema);
    }

    [Fact]
    public void Serializer_SimpleUser_ToTextContainsSameValues()
    {
        var user = new SimpleUser
        {
            Name = "alice",
            Age = 30,
            Active = true
        };

        var generatedText = user.ToTeaLeafText();
        var reflectionText = TeaLeafSerializer.ToText(user);

        // Both should contain the same field values
        Assert.Contains("alice", generatedText);
        Assert.Contains("alice", reflectionText);
        Assert.Contains("30", generatedText);
        Assert.Contains("30", reflectionText);
        Assert.Contains("true", generatedText);
        Assert.Contains("true", reflectionText);
    }

    // ------------------------------------------------------------------
    // Enum serialization
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_Enum_SerializesAsSnakeCase()
    {
        var model = new UserWithEnum
        {
            Name = "admin_user",
            Role = UserRole.Admin
        };

        var text = TeaLeafSerializer.ToText(model);

        Assert.Contains("role:", text);
        Assert.Contains("admin", text);
    }

    // ------------------------------------------------------------------
    // Null handling
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_NullableNull_SerializesAsTilde()
    {
        var model = new NullableFields
        {
            Name = "test",
            Email = null,
            Age = null
        };

        var text = TeaLeafSerializer.ToText(model);

        Assert.Contains("email: ~", text);
    }

    // ------------------------------------------------------------------
    // Regression: Digit-prefix string quoting
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_RoundTrip_DigitPrefixString()
    {
        var user = new SimpleUser
        {
            Name = "44mm Sport Band",
            Age = 30,
            Active = true
        };

        using var doc = TeaLeafSerializer.ToTLDocument(user);
        var restored = TeaLeafSerializer.FromDocument<SimpleUser>(doc);

        Assert.Equal("44mm Sport Band", restored.Name);
    }

    // ------------------------------------------------------------------
    // Regression: Slash character quoting
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_RoundTrip_SlashInString()
    {
        var user = new SimpleUser
        {
            Name = "path/to/resource",
            Age = 25,
            Active = true
        };

        using var doc = TeaLeafSerializer.ToTLDocument(user);
        var restored = TeaLeafSerializer.FromDocument<SimpleUser>(doc);

        Assert.Equal("path/to/resource", restored.Name);
    }

    // ------------------------------------------------------------------
    // Regression: Int-to-float coercion (whole-number doubles)
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_RoundTrip_WholeNumberDouble()
    {
        var product = new Product
        {
            Name = "test",
            Price = 500.0
        };

        using var doc = TeaLeafSerializer.ToTLDocument(product);
        var restored = TeaLeafSerializer.FromDocument<Product>(doc);

        Assert.Equal(500.0, restored.Price, 5);
    }

    // ------------------------------------------------------------------
    // Regression: Collection serialization (indexer filtering)
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_CollectionSerialization_NoIndexerCrash()
    {
        var users = new List<SimpleUser>
        {
            new SimpleUser { Name = "alice", Age = 30, Active = true },
            new SimpleUser { Name = "bob", Age = 25, Active = false }
        };

        // This should NOT throw TargetParameterCountException
        var text = TeaLeafSerializer.ToText<SimpleUser>(users, "users");
        Assert.Contains("alice", text);
        Assert.Contains("bob", text);
    }

    [Fact]
    public void Serializer_CollectionRoundTrip_WorksCorrectly()
    {
        var users = new List<SimpleUser>
        {
            new SimpleUser { Name = "alice", Age = 30, Active = true },
            new SimpleUser { Name = "bob", Age = 25, Active = false }
        };

        using var doc = TeaLeafSerializer.ToTLDocument<SimpleUser>(users, "users");
        var restored = TeaLeafSerializer.FromList<SimpleUser>(doc, "users");

        Assert.Equal(2, restored.Count);
        Assert.Equal("alice", restored[0].Name);
        Assert.Equal(30, restored[0].Age);
        Assert.Equal("bob", restored[1].Name);
        Assert.Equal(25, restored[1].Age);
    }

    // ------------------------------------------------------------------
    // Regression: Collection overload requires explicit type parameter
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_CollectionOverload_RequiresExplicitTypeParameter()
    {
        var users = new List<SimpleUser>
        {
            new SimpleUser { Name = "alice", Age = 30, Active = true },
            new SimpleUser { Name = "bob", Age = 25, Active = false }
        };

        // Explicit type parameter targets the IEnumerable<T> overload
        using var doc = TeaLeafSerializer.ToTLDocument<SimpleUser>(users, "users");

        using var value = doc["users"];
        Assert.NotNull(value);
        Assert.Equal(TLType.Array, value.Type);
        Assert.Equal(2, value.ArrayLength);
    }

    // ------------------------------------------------------------------
    // Nested struct schema and round-trip (reflection)
    // ------------------------------------------------------------------

    [Fact]
    public void Serializer_Schema_IncludesNestedStructs()
    {
        var docText = TeaLeafSerializer.ToDocument(
            new PersonWithAddress
            {
                Name = "x",
                HomeAddress = new Address { Street = "a", City = "b", Zip = "c" }
            });

        Assert.Contains("@struct address", docText);
        Assert.Contains("@struct person_with_address", docText);
    }

    [Fact]
    public void Serializer_RoundTrip_NestedObject()
    {
        var original = new PersonWithAddress
        {
            Name = "bob",
            HomeAddress = new Address { Street = "456 Oak", City = "Portland", Zip = "97201" }
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<PersonWithAddress>(doc);

        Assert.Equal("bob", restored.Name);
        Assert.Equal("456 Oak", restored.HomeAddress.Street);
        Assert.Equal("Portland", restored.HomeAddress.City);
        Assert.Equal("97201", restored.HomeAddress.Zip);
    }

    [Fact]
    public void Serializer_RoundTrip_ListOfNestedObjects()
    {
        var original = new TestOrderWithItems
        {
            OrderId = "ORD-003",
            Items = new List<TestOrderItem>
            {
                new TestOrderItem { ProductName = "Sprocket", Quantity = 5, UnitPrice = 3.50 },
                new TestOrderItem { ProductName = "Cog", Quantity = 10, UnitPrice = 1.25 }
            },
            Total = 30.0
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<TestOrderWithItems>(doc);

        Assert.Equal("ORD-003", restored.OrderId);
        Assert.Equal(2, restored.Items.Count);
        Assert.Equal("Sprocket", restored.Items[0].ProductName);
        Assert.Equal(5, restored.Items[0].Quantity);
        Assert.Equal("Cog", restored.Items[1].ProductName);
        Assert.True(Math.Abs(30.0 - restored.Total) < 0.01);
    }
}
