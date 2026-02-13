using Xunit;

namespace TeaLeaf.Tests;

public class TLDocumentTests
{
    [Fact]
    public void Parse_SimpleValues_ReturnsDocument()
    {
        using var doc = TLDocument.Parse(@"
            name: alice
            age: 30
            active: true
        ");

        Assert.NotNull(doc);
        Assert.Contains("name", doc.Keys);
        Assert.Contains("age", doc.Keys);
        Assert.Contains("active", doc.Keys);
    }

    [Fact]
    public void Parse_String_ReturnsCorrectValue()
    {
        using var doc = TLDocument.Parse("name: alice");
        using var value = doc["name"];

        Assert.NotNull(value);
        Assert.Equal(TLType.String, value.Type);
        Assert.Equal("alice", value.AsString());
    }

    [Fact]
    public void Parse_Integer_ReturnsCorrectValue()
    {
        using var doc = TLDocument.Parse("count: 42");
        using var value = doc["count"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Int, value.Type);
        Assert.Equal(42, value.AsInt());
    }

    [Fact]
    public void Parse_Float_ReturnsCorrectValue()
    {
        using var doc = TLDocument.Parse("price: 19.99");
        using var value = doc["price"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Float, value.Type);
        var floatVal = value.AsFloat();
        Assert.NotNull(floatVal);
        Assert.True(Math.Abs(19.99 - floatVal.Value) < 0.01);
    }

    [Fact]
    public void Parse_Boolean_ReturnsCorrectValue()
    {
        using var doc = TLDocument.Parse(@"
            enabled: true
            disabled: false
        ");

        using var enabled = doc["enabled"];
        using var disabled = doc["disabled"];

        Assert.True(enabled?.AsBool());
        Assert.False(disabled?.AsBool());
    }

    [Fact]
    public void Parse_Array_ReturnsCorrectValues()
    {
        using var doc = TLDocument.Parse("items: [1, 2, 3]");
        using var value = doc["items"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Array, value.Type);
        Assert.Equal(3, value.ArrayLength);

        using var first = value[0];
        Assert.Equal(1, first?.AsInt());
    }

    [Fact]
    public void Parse_Object_ReturnsCorrectValues()
    {
        using var doc = TLDocument.Parse("user: {name: bob, age: 25}");
        using var value = doc["user"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Object, value.Type);

        using var name = value["name"];
        Assert.Equal("bob", name?.AsString());
    }

    [Fact]
    public void Parse_BytesLiteral_ReturnsCorrectValue()
    {
        using var doc = TLDocument.Parse("payload: b\"cafef00d\"\nempty: b\"\"");

        using var payload = doc["payload"];
        Assert.NotNull(payload);
        Assert.Equal(TLType.Bytes, payload.Type);
        var bytes = payload.AsBytes();
        Assert.NotNull(bytes);
        Assert.Equal(new byte[] { 0xca, 0xfe, 0xf0, 0x0d }, bytes);

        using var empty = doc["empty"];
        Assert.NotNull(empty);
        Assert.Equal(TLType.Bytes, empty.Type);
        var emptyBytes = empty.AsBytes();
        Assert.NotNull(emptyBytes);
        Assert.Empty(emptyBytes);
    }

    [Fact]
    public void Parse_BytesLiteral_TextRoundTrip()
    {
        using var doc = TLDocument.Parse("data: b\"cafef00d\"");
        var text = doc.ToText();

        Assert.NotNull(text);
        Assert.Contains("b\"cafef00d\"", text);

        // Re-parse and verify
        using var doc2 = TLDocument.Parse(text);
        using var data = doc2["data"];
        Assert.NotNull(data);
        Assert.Equal(TLType.Bytes, data.Type);
        Assert.Equal(new byte[] { 0xca, 0xfe, 0xf0, 0x0d }, data.AsBytes());
    }

    [Fact]
    public void Parse_InvalidSyntax_ThrowsException()
    {
        Assert.Throws<TLException>(() => TLDocument.Parse("invalid: [unclosed"));
    }

    [Fact]
    public void Parse_InvalidSyntax_ExceptionHasDetailedMessage()
    {
        var ex = Assert.Throws<TLException>(() => TLDocument.Parse("invalid: [unclosed"));

        // Error message should contain "Parse error" and describe the issue
        Assert.Contains("Parse error", ex.Message);
    }

    [Fact]
    public void FromJson_InvalidJson_ExceptionHasDetailedMessage()
    {
        var ex = Assert.Throws<TLException>(() => TLDocument.FromJson("not valid json"));

        // Error message should mention JSON and the problem
        Assert.Contains("JSON", ex.Message);
    }

    [Fact]
    public void TryParse_ValidInput_ReturnsTrue()
    {
        var success = TLDocument.TryParse("value: 123", out var doc);

        Assert.True(success);
        Assert.NotNull(doc);
        doc?.Dispose();
    }

    [Fact]
    public void TryParse_InvalidInput_ReturnsFalse()
    {
        var success = TLDocument.TryParse("invalid: [", out var doc);

        Assert.False(success);
        Assert.Null(doc);
    }

    [Fact]
    public void Get_NonExistentKey_ReturnsNull()
    {
        using var doc = TLDocument.Parse("key: value");
        var value = doc["nonexistent"];

        Assert.Null(value);
    }

    [Fact]
    public void ContainsKey_ExistingKey_ReturnsTrue()
    {
        using var doc = TLDocument.Parse("key: value");

        Assert.True(doc.ContainsKey("key"));
        Assert.False(doc.ContainsKey("other"));
    }

    [Fact]
    public void ToText_ReturnsValidTeaLeafFormat()
    {
        using var doc = TLDocument.Parse("greeting: hello");
        var text = doc.ToText();

        Assert.NotNull(text);
        Assert.Contains("greeting", text);
        Assert.Contains("hello", text);
    }

    [Fact]
    public void ToText_Compact_RemovesInsignificantWhitespace()
    {
        using var doc = TLDocument.Parse("name: alice\nage: 30");
        var compact = doc.ToText(compact: true, compactFloats: false);

        Assert.NotNull(compact);
        Assert.Contains("name:alice", compact);
        Assert.Contains("age:30", compact);
    }

    [Fact]
    public void ToText_Compact_WithSchemas_IsSmallerThanPretty()
    {
        const string json = @"{""users"": [{""id"": 1, ""name"": ""alice""}, {""id"": 2, ""name"": ""bob""}]}";
        using var doc = TLDocument.FromJson(json);

        var pretty = doc.ToText();
        var compact = doc.ToText(compact: true, compactFloats: false);

        Assert.True(compact.Length < pretty.Length,
            $"Compact ({compact.Length}) should be smaller than pretty ({pretty.Length})");
    }

    [Fact]
    public void ToText_Compact_RoundTrips()
    {
        const string json = @"{""name"": ""Alice Smith"", ""items"": [1, 2, 3]}";
        using var doc = TLDocument.FromJson(json);

        var compact = doc.ToText(compact: true, compactFloats: false);
        using var reparsed = TLDocument.Parse(compact);

        var json1 = doc.ToJson();
        var json2 = reparsed.ToJson();
        Assert.Equal(json1, json2);
    }

    [Fact]
    public void ToText_IgnoreSchemas_ExcludesSchemas()
    {
        const string json = @"{""users"": [{""id"": 1, ""name"": ""alice""}, {""id"": 2, ""name"": ""bob""}]}";
        using var doc = TLDocument.FromJson(json);

        var dataOnly = doc.ToText(ignoreSchemas: true);

        Assert.DoesNotContain("@struct", dataOnly);
        Assert.NotNull(dataOnly);
        Assert.True(dataOnly.Length > 0);
    }

    // ==========================================================================
    // JSON Conversion Tests
    // ==========================================================================

    [Fact]
    public void FromJson_SimpleObject_ReturnsDocument()
    {
        using var doc = TLDocument.FromJson(@"{""name"": ""alice"", ""age"": 30}");

        Assert.NotNull(doc);
        Assert.Contains("name", doc.Keys);
        Assert.Contains("age", doc.Keys);

        using var name = doc["name"];
        Assert.Equal("alice", name?.AsString());

        using var age = doc["age"];
        Assert.Equal(30, age?.AsInt());
    }

    [Fact]
    public void FromJson_NestedObject_ReturnsDocument()
    {
        using var doc = TLDocument.FromJson(@"{""user"": {""name"": ""bob"", ""active"": true}}");

        using var user = doc["user"];
        Assert.NotNull(user);
        Assert.Equal(TLType.Object, user.Type);

        using var name = user["name"];
        Assert.Equal("bob", name?.AsString());

        using var active = user["active"];
        Assert.True(active?.AsBool());
    }

    [Fact]
    public void FromJson_Array_ReturnsDocument()
    {
        using var doc = TLDocument.FromJson(@"{""items"": [1, 2, 3, 4, 5]}");

        using var items = doc["items"];
        Assert.NotNull(items);
        Assert.Equal(TLType.Array, items.Type);
        Assert.Equal(5, items.ArrayLength);

        using var first = items[0];
        Assert.Equal(1, first?.AsInt());
    }

    [Fact]
    public void FromJson_InvalidJson_ThrowsException()
    {
        Assert.Throws<TLException>(() => TLDocument.FromJson("not valid json"));
    }

    [Fact]
    public void TryFromJson_ValidInput_ReturnsTrue()
    {
        var success = TLDocument.TryFromJson(@"{""key"": ""value""}", out var doc);

        Assert.True(success);
        Assert.NotNull(doc);
        doc?.Dispose();
    }

    [Fact]
    public void TryFromJson_InvalidInput_ReturnsFalse()
    {
        var success = TLDocument.TryFromJson("{invalid", out var doc);

        Assert.False(success);
        Assert.Null(doc);
    }

    [Fact]
    public void ToJson_SimpleDocument_ReturnsValidJson()
    {
        using var doc = TLDocument.Parse(@"
            name: alice
            age: 30
        ");

        var json = doc.ToJson();

        Assert.NotNull(json);
        Assert.Contains("name", json);
        Assert.Contains("alice", json);
        Assert.Contains("age", json);
        Assert.Contains("30", json);
    }

    [Fact]
    public void ToJsonCompact_ReturnsMinifiedJson()
    {
        using var doc = TLDocument.Parse("name: alice");

        var json = doc.ToJsonCompact();

        Assert.NotNull(json);
        Assert.DoesNotContain("\n", json);
        Assert.Contains("name", json);
        Assert.Contains("alice", json);
    }

    [Fact]
    public void JsonRoundTrip_PreservesData()
    {
        using var original = TLDocument.Parse(@"
            title: test
            count: 42
            items: [a, b, c]
        ");

        var json = original.ToJson();
        using var restored = TLDocument.FromJson(json);

        using var title = restored["title"];
        Assert.Equal("test", title?.AsString());

        using var count = restored["count"];
        Assert.Equal(42, count?.AsInt());

        using var items = restored["items"];
        Assert.Equal(3, items?.ArrayLength);
    }

    [Fact]
    public void JsonRoundTrip_ComplexNestedStructure()
    {
        // Complex nested JSON with various data types
        const string complexJson = @"{
            ""company"": {
                ""name"": ""Acme Corporation"",
                ""founded"": 1985,
                ""active"": true,
                ""rating"": 4.8,
                ""headquarters"": {
                    ""city"": ""San Francisco"",
                    ""country"": ""USA"",
                    ""coordinates"": {
                        ""lat"": 37.7749,
                        ""lng"": -122.4194
                    }
                },
                ""departments"": [
                    {
                        ""name"": ""Engineering"",
                        ""headcount"": 150,
                        ""teams"": [""Backend"", ""Frontend"", ""DevOps"", ""QA""]
                    },
                    {
                        ""name"": ""Sales"",
                        ""headcount"": 75,
                        ""teams"": [""Enterprise"", ""SMB"", ""Partners""]
                    },
                    {
                        ""name"": ""Marketing"",
                        ""headcount"": 40,
                        ""teams"": [""Content"", ""Digital"", ""Events""]
                    }
                ],
                ""employees"": [
                    {
                        ""id"": 1,
                        ""name"": ""Alice Johnson"",
                        ""email"": ""alice@acme.com"",
                        ""roles"": [""admin"", ""developer""],
                        ""metadata"": {
                            ""hired"": ""2020-01-15"",
                            ""level"": 5,
                            ""remote"": true
                        }
                    },
                    {
                        ""id"": 2,
                        ""name"": ""Bob Smith"",
                        ""email"": ""bob@acme.com"",
                        ""roles"": [""developer""],
                        ""metadata"": {
                            ""hired"": ""2021-06-01"",
                            ""level"": 3,
                            ""remote"": false
                        }
                    }
                ],
                ""settings"": {
                    ""features"": {
                        ""darkMode"": true,
                        ""notifications"": true,
                        ""analytics"": false
                    },
                    ""limits"": {
                        ""maxUsers"": 1000,
                        ""maxStorage"": 5000000000,
                        ""maxRequests"": 10000
                    }
                }
            },
            ""version"": ""2.0.0"",
            ""tags"": [""enterprise"", ""saas"", ""cloud""],
            ""matrix"": [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
        }";

        // Parse JSON to TeaLeaf
        using var fromJson = TLDocument.FromJson(complexJson);
        Assert.NotNull(fromJson);

        // JSON -> JSON round trip (skip TeaLeaf text for now)
        var roundTripJson = fromJson.ToJson();
        Assert.NotNull(roundTripJson);

        // Parse the round-tripped JSON
        using var final = TLDocument.FromJson(roundTripJson);
        Assert.NotNull(final);

        // Validate deeply nested values
        using var company = final["company"];
        Assert.NotNull(company);
        Assert.Equal(TLType.Object, company.Type);

        using var companyName = company["name"];
        Assert.Equal("Acme Corporation", companyName?.AsString());

        using var founded = company["founded"];
        Assert.Equal(1985, founded?.AsInt());

        using var active = company["active"];
        Assert.True(active?.AsBool());

        using var rating = company["rating"];
        var ratingValue = rating?.AsFloat();
        Assert.NotNull(ratingValue);
        Assert.True(Math.Abs(4.8 - ratingValue.Value) < 0.1);

        // Validate nested headquarters
        using var hq = company["headquarters"];
        Assert.NotNull(hq);

        using var city = hq["city"];
        Assert.Equal("San Francisco", city?.AsString());

        using var coords = hq["coordinates"];
        Assert.NotNull(coords);

        using var lat = coords["lat"];
        var latValue = lat?.AsFloat();
        Assert.NotNull(latValue);
        Assert.True(Math.Abs(37.7749 - latValue.Value) < 0.0001);

        // Validate array of objects (departments)
        using var departments = company["departments"];
        Assert.NotNull(departments);
        Assert.Equal(3, departments.ArrayLength);

        using var firstDept = departments[0];
        Assert.NotNull(firstDept);

        using var deptName = firstDept["name"];
        Assert.Equal("Engineering", deptName?.AsString());

        using var headcount = firstDept["headcount"];
        Assert.Equal(150, headcount?.AsInt());

        using var teams = firstDept["teams"];
        Assert.Equal(4, teams?.ArrayLength);

        // Validate employees array
        using var employees = company["employees"];
        Assert.Equal(2, employees?.ArrayLength);

        using var firstEmployee = employees?[0];
        Assert.NotNull(firstEmployee);

        using var empName = firstEmployee["name"];
        Assert.Equal("Alice Johnson", empName?.AsString());

        using var roles = firstEmployee["roles"];
        Assert.Equal(2, roles?.ArrayLength);

        using var metadata = firstEmployee["metadata"];
        Assert.NotNull(metadata);

        using var level = metadata["level"];
        Assert.Equal(5, level?.AsInt());

        using var remote = metadata["remote"];
        Assert.True(remote?.AsBool());

        // Validate nested settings
        using var settings = company["settings"];
        Assert.NotNull(settings);

        using var features = settings["features"];
        Assert.NotNull(features);

        using var darkMode = features["darkMode"];
        Assert.True(darkMode?.AsBool());

        using var limits = settings["limits"];
        Assert.NotNull(limits);

        using var maxUsers = limits["maxUsers"];
        Assert.Equal(1000, maxUsers?.AsInt());

        // Validate top-level values
        using var version = final["version"];
        Assert.Equal("2.0.0", version?.AsString());

        using var tags = final["tags"];
        Assert.Equal(3, tags?.ArrayLength);

        // Validate nested array (matrix)
        using var matrix = final["matrix"];
        Assert.NotNull(matrix);
        Assert.Equal(3, matrix.ArrayLength);

        using var firstRow = matrix[0];
        Assert.Equal(3, firstRow?.ArrayLength);

        using var firstCell = firstRow?[0];
        Assert.Equal(1, firstCell?.AsInt());

        using var lastRow = matrix[2];
        using var lastCell = lastRow?[2];
        Assert.Equal(9, lastCell?.AsInt());
    }

    // ==========================================================================
    // Schema Inference Tests
    // ==========================================================================

    [Fact]
    public void FromJson_UniformObjectArray_InfersSchema()
    {
        // When JSON has uniform object arrays, schema inference should work
        const string json = @"{""users"": [{""name"": ""alice"", ""age"": 30}, {""name"": ""bob"", ""age"": 25}]}";
        using var doc = TLDocument.FromJson(json);

        // Data should be accessible
        using var users = doc["users"];
        Assert.NotNull(users);
        Assert.Equal(2, users.ArrayLength);

        using var firstUser = users[0];
        Assert.NotNull(firstUser);

        using var name = firstUser["name"];
        Assert.Equal("alice", name?.AsString());

        using var age = firstUser["age"];
        Assert.Equal(30, age?.AsInt());
    }

    [Fact]
    public void FromJson_NestedArrays_InfersNestedSchemas()
    {
        const string json = @"{
            ""orders"": [
                {""id"": 1, ""items"": [{""sku"": ""A"", ""qty"": 2}, {""sku"": ""B"", ""qty"": 1}]},
                {""id"": 2, ""items"": [{""sku"": ""C"", ""qty"": 3}]}
            ]
        }";
        using var doc = TLDocument.FromJson(json);

        // Access nested data
        using var orders = doc["orders"];
        Assert.NotNull(orders);
        Assert.Equal(2, orders.ArrayLength);

        using var firstOrder = orders[0];
        Assert.NotNull(firstOrder);

        using var items = firstOrder["items"];
        Assert.NotNull(items);
        Assert.Equal(2, items.ArrayLength);

        using var firstItem = items[0];
        Assert.NotNull(firstItem);

        using var sku = firstItem["sku"];
        Assert.Equal("A", sku?.AsString());

        using var qty = firstItem["qty"];
        Assert.Equal(2, qty?.AsInt());
    }

    [Fact]
    public void FromJson_ToText_IncludesSchemaDefinitions()
    {
        const string json = @"{""products"": [{""name"": ""Widget"", ""price"": 9.99}, {""name"": ""Gadget"", ""price"": 19.99}]}";
        using var doc = TLDocument.FromJson(json);

        var tlText = doc.ToText();
        Assert.NotNull(tlText);

        // Should contain struct definition
        Assert.Contains("@struct product", tlText);
        Assert.Contains("name:", tlText);
        Assert.Contains("price:", tlText);

        // Should contain @table directive
        Assert.Contains("@table product", tlText);
    }

    [Fact]
    public void FromJson_Roundtrip_PreservesSchemaInference()
    {
        const string json = @"{""items"": [{""id"": 1, ""name"": ""A""}, {""id"": 2, ""name"": ""B""}]}";
        using var doc = TLDocument.FromJson(json);

        // Convert to TL text with schemas
        var tlText = doc.ToText();
        Assert.NotNull(tlText);

        // Parse the TL text back
        using var parsed = TLDocument.Parse(tlText);
        Assert.NotNull(parsed);

        // Data should be preserved
        using var items = parsed["items"];
        Assert.NotNull(items);
        Assert.Equal(2, items.ArrayLength);

        using var firstItem = items[0];
        Assert.NotNull(firstItem);

        using var id = firstItem["id"];
        Assert.Equal(1, id?.AsInt());

        using var name = firstItem["name"];
        Assert.Equal("A", name?.AsString());
    }

    [Fact]
    public void FromJson_NullableFields_HandledCorrectly()
    {
        const string json = @"{""users"": [{""name"": ""alice"", ""email"": ""a@test.com""}, {""name"": ""bob"", ""email"": null}]}";
        using var doc = TLDocument.FromJson(json);

        using var users = doc["users"];
        Assert.NotNull(users);

        // First user has email
        using var firstUser = users[0];
        using var email1 = firstUser?["email"];
        Assert.Equal("a@test.com", email1?.AsString());

        // Second user has null email
        using var secondUser = users[1];
        using var email2 = secondUser?["email"];
        Assert.NotNull(email2);
        Assert.Equal(TLType.Null, email2.Type);
    }

    [Fact]
    public void FromJson_SpecialCharacters_QuotedCorrectly()
    {
        // Test that special characters are handled correctly through roundtrip
        const string json = @"{""items"": [
            {""category"": ""Electronics/Audio"", ""email"": ""test@example.com"", ""path"": ""a.b.c""}
        ]}";
        using var doc = TLDocument.FromJson(json);

        var tlText = doc.ToText();
        Assert.NotNull(tlText);

        // Parse back
        using var parsed = TLDocument.Parse(tlText);
        using var items = parsed["items"];
        using var item = items?[0];

        using var category = item?["category"];
        Assert.Equal("Electronics/Audio", category?.AsString());

        using var email = item?["email"];
        Assert.Equal("test@example.com", email?.AsString());

        using var path = item?["path"];
        Assert.Equal("a.b.c", path?.AsString());
    }

    // ==========================================================================
    // Schema Access Tests (Action #1)
    // ==========================================================================

    [Fact]
    public void Schemas_ParsedDocumentWithSchemas_ReturnsSchemas()
    {
        using var doc = TLDocument.Parse(@"
            @struct user (name: string, age: int, active: bool)

            users: @table user [
                (alice, 30, true)
                (bob, 25, false)
            ]
        ");

        Assert.Equal(1, doc.SchemaCount);
        Assert.Single(doc.Schemas);

        var schema = doc.Schemas[0];
        Assert.Equal("user", schema.Name);
        Assert.Equal(3, schema.Fields.Count);
        Assert.Equal("name", schema.Fields[0].Name);
        Assert.Equal("string", schema.Fields[0].Type);
        Assert.Equal("age", schema.Fields[1].Name);
        Assert.Equal("int", schema.Fields[1].Type);
        Assert.Equal("active", schema.Fields[2].Name);
        Assert.Equal("bool", schema.Fields[2].Type);
    }

    [Fact]
    public void GetSchema_ByName_ReturnsCorrectSchema()
    {
        using var doc = TLDocument.Parse(@"
            @struct item (sku: string, qty: int, price: float)
            @struct order (id: string, total: float)

            items: @table item [
                (A, 2, 9.99)
            ]
            order: { id: ORD-001, total: 9.99 }
        ");

        Assert.Equal(2, doc.SchemaCount);

        var item = doc.GetSchema("item");
        Assert.NotNull(item);
        Assert.Equal("item", item!.Name);
        Assert.Equal(3, item.Fields.Count);
        Assert.Equal("sku", item.Fields[0].Name);

        var order = doc.GetSchema("order");
        Assert.NotNull(order);
        Assert.Equal("order", order!.Name);
        Assert.Equal(2, order.Fields.Count);

        var notFound = doc.GetSchema("nonexistent");
        Assert.Null(notFound);
    }

    [Fact]
    public void SchemaCount_NoSchemas_ReturnsZero()
    {
        using var doc = TLDocument.Parse("name: alice\nage: 30");

        Assert.Equal(0, doc.SchemaCount);
        Assert.Empty(doc.Schemas);
    }

    [Fact]
    public void Schemas_FromJson_ReturnsInferredSchemas()
    {
        const string json = @"{""users"": [{""name"": ""alice"", ""age"": 30}, {""name"": ""bob"", ""age"": 25}]}";
        using var doc = TLDocument.FromJson(json);

        // FromJson infers schemas for uniform object arrays
        Assert.True(doc.SchemaCount > 0, "Expected at least one inferred schema");
        Assert.True(doc.Schemas.Count > 0);

        // The inferred schema should have name and age fields
        var schema = doc.Schemas[0];
        Assert.True(schema.HasField("name"));
        Assert.True(schema.HasField("age"));
    }

    [Fact]
    public void Schemas_NullableAndArrayFields_ReportCorrectly()
    {
        using var doc = TLDocument.Parse(@"
            @struct record (name: string, email: string?, tags: []string)

            data: @table record [
                (alice, alice_email, [dev, ops])
            ]
        ");

        Assert.Equal(1, doc.SchemaCount);
        var schema = doc.GetSchema("record");
        Assert.NotNull(schema);

        var nameField = schema!.GetField("name");
        Assert.NotNull(nameField);
        Assert.False(nameField!.IsNullable);
        Assert.False(nameField.IsArray);

        var emailField = schema.GetField("email");
        Assert.NotNull(emailField);
        Assert.True(emailField!.IsNullable);
        Assert.False(emailField.IsArray);

        var tagsField = schema.GetField("tags");
        Assert.NotNull(tagsField);
        Assert.False(tagsField!.IsNullable);
        Assert.True(tagsField.IsArray);
    }

    [Fact]
    public void FromJson_MixedArrayTypes_StaysAsAny()
    {
        // When array elements have different structures, they should stay as 'any' type
        const string json = @"{
            ""data"": [
                {""type"": ""user"", ""name"": ""alice""},
                {""type"": ""product"", ""price"": 9.99}
            ]
        }";
        using var doc = TLDocument.FromJson(json);

        using var data = doc["data"];
        Assert.NotNull(data);
        Assert.Equal(2, data.ArrayLength);

        // Both items should still be accessible as objects
        using var first = data[0];
        using var firstType = first?["type"];
        Assert.Equal("user", firstType?.AsString());

        using var second = data[1];
        using var secondType = second?["type"];
        Assert.Equal("product", secondType?.AsString());

        using var price = second?["price"];
        var priceValue = price?.AsFloat();
        Assert.NotNull(priceValue);
        Assert.True(Math.Abs(9.99 - priceValue.Value) < 0.01);
    }
}
