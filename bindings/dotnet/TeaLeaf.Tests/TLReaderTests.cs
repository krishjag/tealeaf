using System.Text.Json;
using Xunit;

namespace TeaLeaf.Tests;

public class TLReaderTests
{
    private string _tempDir = Path.GetTempPath();

    // ==========================================================================
    // Binary File Reading Tests
    // ==========================================================================

    [Fact]
    public void Open_CompiledFile_ReturnsReader()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_reader.tlbx");

        try
        {
            // Create a binary file from TeaLeaf text
            using (var doc = TLDocument.Parse(@"
                name: alice
                age: 30
                active: true
            "))
            {
                doc.Compile(tlbxPath);
            }

            // Open and read the binary file
            using var reader = TLReader.Open(tlbxPath);
            Assert.NotNull(reader);

            using var name = reader["name"];
            Assert.NotNull(name);
            Assert.Equal("alice", name.AsString());

            using var age = reader["age"];
            Assert.Equal(30, age?.AsInt());

            using var active = reader["active"];
            Assert.True(active?.AsBool());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void TryOpen_ValidFile_ReturnsTrue()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_tryopen.tlbx");

        try
        {
            using (var doc = TLDocument.Parse("key: value"))
            {
                doc.Compile(tlbxPath);
            }

            var success = TLReader.TryOpen(tlbxPath, out var reader);
            Assert.True(success);
            Assert.NotNull(reader);
            reader?.Dispose();
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void TryOpen_InvalidFile_ReturnsFalse()
    {
        var success = TLReader.TryOpen("nonexistent.tlbx", out var reader);
        Assert.False(success);
        Assert.Null(reader);
    }

    [Fact]
    public void Open_NonexistentFile_ExceptionHasDetailedMessage()
    {
        var ex = Assert.Throws<TLException>(() => TLReader.Open("nonexistent_file.tlbx"));

        // Error message should mention the file path and the problem
        Assert.Contains("nonexistent_file.tlbx", ex.Message);
        Assert.Contains("Failed to open", ex.Message);
    }

    [Fact]
    public void Open_InvalidBinaryFile_ExceptionHasDetailedMessage()
    {
        var tempFile = Path.Combine(_tempDir, "invalid_binary.tlbx");
        try
        {
            // Write invalid binary data (not a valid tlbx)
            File.WriteAllText(tempFile, "not a valid tlbx file");

            var ex = Assert.Throws<TLException>(() => TLReader.Open(tempFile));

            // Error message should describe the problem
            Assert.Contains(tempFile, ex.Message);
        }
        finally
        {
            if (File.Exists(tempFile))
                File.Delete(tempFile);
        }
    }

    // ==========================================================================
    // JSON Conversion Tests (TLBX -> JSON)
    // ==========================================================================

    [Fact]
    public void ToJson_CompiledFile_ReturnsValidJson()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_tojson.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                name: alice
                age: 30
                items: [1, 2, 3]
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            Assert.NotNull(json);
            Assert.Contains("name", json);
            Assert.Contains("alice", json);
            Assert.Contains("age", json);
            Assert.Contains("30", json);
            Assert.Contains("items", json);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJsonCompact_ReturnsMinifiedJson()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_compact.tlbx");

        try
        {
            using (var doc = TLDocument.Parse("name: alice"))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJsonCompact();

            Assert.NotNull(json);
            Assert.DoesNotContain("\n", json);
            Assert.Contains("name", json);
            Assert.Contains("alice", json);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void GetAsJson_SingleKey_ReturnsJsonValue()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_getasjson.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                config: { debug: true, level: info }
                items: [1, 2, 3]
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);

            var configJson = reader.GetAsJson("config");
            Assert.NotNull(configJson);
            Assert.Contains("debug", configJson);
            Assert.Contains("true", configJson);

            var itemsJson = reader.GetAsJson("items");
            Assert.NotNull(itemsJson);
            Assert.Contains("1", itemsJson);
            Assert.Contains("2", itemsJson);
            Assert.Contains("3", itemsJson);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void GetAsJson_NonExistentKey_ReturnsNull()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_getasjson_null.tlbx");

        try
        {
            using (var doc = TLDocument.Parse("key: value"))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.GetAsJson("nonexistent");
            Assert.Null(json);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    // ==========================================================================
    // JSON to TLBX Conversion Tests
    // ==========================================================================

    [Fact]
    public void CreateFromJson_ValidJson_CreatesTlbxFile()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_fromjson.tlbx");

        try
        {
            TLReader.CreateFromJson(@"{""name"": ""alice"", ""age"": 30}", tlbxPath);

            Assert.True(File.Exists(tlbxPath));

            using var reader = TLReader.Open(tlbxPath);
            using var name = reader["name"];
            Assert.Equal("alice", name?.AsString());

            using var age = reader["age"];
            Assert.Equal(30, age?.AsInt());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void TryCreateFromJson_ValidJson_ReturnsTrue()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_trycreate.tlbx");

        try
        {
            var success = TLReader.TryCreateFromJson(@"{""key"": ""value""}", tlbxPath);
            Assert.True(success);
            Assert.True(File.Exists(tlbxPath));
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void TryCreateFromJson_InvalidJson_ReturnsFalse()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_trycreate_invalid.tlbx");

        try
        {
            var success = TLReader.TryCreateFromJson("{invalid json", tlbxPath);
            Assert.False(success);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    // ==========================================================================
    // JSON Round-Trip Tests
    // ==========================================================================

    [Fact]
    public void JsonRoundTrip_ThroughTlbx_PreservesData()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_roundtrip.tlbx");

        try
        {
            // JSON -> TLBX
            const string originalJson = @"{
                ""name"": ""Alice"",
                ""age"": 30,
                ""active"": true,
                ""scores"": [95.5, 88.0, 92.3]
            }";

            TLReader.CreateFromJson(originalJson, tlbxPath);

            // TLBX -> JSON
            using var reader = TLReader.Open(tlbxPath);
            var resultJson = reader.ToJson();

            // Verify data preserved
            Assert.Contains("name", resultJson);
            Assert.Contains("Alice", resultJson);
            Assert.Contains("age", resultJson);
            Assert.Contains("30", resultJson);
            Assert.Contains("active", resultJson);
            Assert.Contains("true", resultJson);
            Assert.Contains("scores", resultJson);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_NestedStructures_PreservesHierarchy()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_nested.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                user: {
                    name: alice
                    profile: {
                        bio: developer
                        settings: {
                            theme: dark
                            notifications: true
                        }
                    }
                }
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            Assert.Contains("user", json);
            Assert.Contains("name", json);
            Assert.Contains("alice", json);
            Assert.Contains("profile", json);
            Assert.Contains("bio", json);
            Assert.Contains("settings", json);
            Assert.Contains("theme", json);
            Assert.Contains("dark", json);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    // ==========================================================================
    // Structural JSON Output Tests
    // ==========================================================================

    [Fact]
    public void ToJson_Ref_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_ref_json.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                base: {host: localhost}
                config: !base
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            // Parse JSON and verify structure
            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            // config should be an object with $ref property
            Assert.True(root.TryGetProperty("config", out var config));
            Assert.Equal(JsonValueKind.Object, config.ValueKind);
            Assert.True(config.TryGetProperty("$ref", out var refValue));
            Assert.Equal(JsonValueKind.String, refValue.ValueKind);
            Assert.Equal("base", refValue.GetString());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_Tagged_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_tagged_json.tlbx");

        try
        {
            using (var doc = TLDocument.Parse("status: :error 404"))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            // Parse JSON and verify structure
            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            // status should be an object with $tag and $value properties
            Assert.True(root.TryGetProperty("status", out var status));
            Assert.Equal(JsonValueKind.Object, status.ValueKind);

            Assert.True(status.TryGetProperty("$tag", out var tag));
            Assert.Equal(JsonValueKind.String, tag.ValueKind);
            Assert.Equal("error", tag.GetString());

            Assert.True(status.TryGetProperty("$value", out var value));
            Assert.Equal(JsonValueKind.Number, value.ValueKind);
            Assert.Equal(404, value.GetInt64());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_Map_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_map_json.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                lookup: @map {
                    1: one,
                    2: two
                }
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            // Parse JSON and verify structure
            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            // lookup should be an array of [key, value] pairs
            Assert.True(root.TryGetProperty("lookup", out var lookup));
            Assert.Equal(JsonValueKind.Array, lookup.ValueKind);
            Assert.Equal(2, lookup.GetArrayLength());

            // First pair: [1, "one"]
            var pair0 = lookup[0];
            Assert.Equal(JsonValueKind.Array, pair0.ValueKind);
            Assert.Equal(2, pair0.GetArrayLength());
            Assert.Equal(1, pair0[0].GetInt64());
            Assert.Equal("one", pair0[1].GetString());

            // Second pair: [2, "two"]
            var pair1 = lookup[1];
            Assert.Equal(JsonValueKind.Array, pair1.ValueKind);
            Assert.Equal(2, pair1.GetArrayLength());
            Assert.Equal(2, pair1[0].GetInt64());
            Assert.Equal("two", pair1[1].GetString());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_Object_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_object_json.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                user: {
                    name: alice,
                    age: 30,
                    active: true
                }
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            // Parse JSON and verify structure
            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            Assert.True(root.TryGetProperty("user", out var user));
            Assert.Equal(JsonValueKind.Object, user.ValueKind);

            Assert.True(user.TryGetProperty("name", out var name));
            Assert.Equal("alice", name.GetString());

            Assert.True(user.TryGetProperty("age", out var age));
            Assert.Equal(30, age.GetInt64());

            Assert.True(user.TryGetProperty("active", out var active));
            Assert.True(active.GetBoolean());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_Array_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_array_json.tlbx");

        try
        {
            using (var doc = TLDocument.Parse("items: [1, 2, 3, 4, 5]"))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            // Parse JSON and verify structure
            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            Assert.True(root.TryGetProperty("items", out var items));
            Assert.Equal(JsonValueKind.Array, items.ValueKind);
            Assert.Equal(5, items.GetArrayLength());

            for (int i = 0; i < 5; i++)
            {
                Assert.Equal(i + 1, items[i].GetInt64());
            }
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_MixedTypes_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_mixed_json.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                config: {
                    name: app,
                    version: 1,
                    pi: 3.14,
                    enabled: true,
                    nothing: ~,
                    tags: [a, b, c]
                }
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.ToJson();

            // Parse JSON and verify structure
            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            Assert.True(root.TryGetProperty("config", out var config));
            Assert.Equal(JsonValueKind.Object, config.ValueKind);

            // String
            Assert.True(config.TryGetProperty("name", out var name));
            Assert.Equal(JsonValueKind.String, name.ValueKind);
            Assert.Equal("app", name.GetString());

            // Integer
            Assert.True(config.TryGetProperty("version", out var version));
            Assert.Equal(JsonValueKind.Number, version.ValueKind);
            Assert.Equal(1, version.GetInt64());

            // Float
            Assert.True(config.TryGetProperty("pi", out var pi));
            Assert.Equal(JsonValueKind.Number, pi.ValueKind);
            Assert.Equal(3.14, pi.GetDouble(), 2);

            // Boolean
            Assert.True(config.TryGetProperty("enabled", out var enabled));
            Assert.Equal(JsonValueKind.True, enabled.ValueKind);

            // Null
            Assert.True(config.TryGetProperty("nothing", out var nothing));
            Assert.Equal(JsonValueKind.Null, nothing.ValueKind);

            // Array
            Assert.True(config.TryGetProperty("tags", out var tags));
            Assert.Equal(JsonValueKind.Array, tags.ValueKind);
            Assert.Equal(3, tags.GetArrayLength());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void GetAsJson_Ref_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_getasjson_ref.tlbx");

        try
        {
            using (var doc = TLDocument.Parse(@"
                base: {x: 1}
                ptr: !base
            "))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.GetAsJson("ptr");
            Assert.NotNull(json);

            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            Assert.Equal(JsonValueKind.Object, root.ValueKind);
            Assert.True(root.TryGetProperty("$ref", out var refValue));
            Assert.Equal("base", refValue.GetString());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void GetAsJson_Tagged_HasCorrectStructure()
    {
        var tlbxPath = Path.Combine(_tempDir, "test_getasjson_tagged.tlbx");

        try
        {
            using (var doc = TLDocument.Parse("result: :ok done"))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var json = reader.GetAsJson("result");
            Assert.NotNull(json);

            using var jsonDoc = JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;

            Assert.Equal(JsonValueKind.Object, root.ValueKind);
            Assert.True(root.TryGetProperty("$tag", out var tag));
            Assert.Equal("ok", tag.GetString());
            Assert.True(root.TryGetProperty("$value", out var value));
            Assert.Equal("done", value.GetString());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    // ==========================================================================
    // Schema Inference → Binary Compilation Tests
    // ==========================================================================
    // These tests verify that FromJson (with schema inference) → Compile → Read
    // works correctly, including edge cases where schema inference produces
    // pseudo-types like 'any' for heterogeneous nested objects.

    [Fact]
    public void FromJson_HeterogeneousNestedObjects_CompileSucceeds()
    {
        // Regression: FromJson infers 'any' pseudo-type for fields whose nested objects
        // have varying shapes. The binary encoder must fall back to generic encoding
        // instead of erroring with "schema-typed field 'any' requires a schema".
        var tlbxPath = Path.Combine(_tempDir, "test_any_type.tlbx");

        try
        {
            const string json = @"[
                {""name"": ""alice"", ""meta"": {""x"": 1}},
                {""name"": ""bob"",   ""meta"": {""y"": ""two"", ""z"": true}}
            ]";

            TLReader.CreateFromJson(json, tlbxPath);

            Assert.True(File.Exists(tlbxPath));

            using var reader = TLReader.Open(tlbxPath);
            var resultJson = reader.ToJson();
            Assert.Contains("alice", resultJson);
            Assert.Contains("bob", resultJson);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void FromJson_MixedStructureArrays_CompileRoundTrips()
    {
        // Array elements with different field sets → inferred as 'any', not a schema.
        // Must compile and round-trip correctly through binary.
        var tlbxPath = Path.Combine(_tempDir, "test_mixed_compile.tlbx");

        try
        {
            const string json = @"{
                ""data"": [
                    {""type"": ""user"", ""name"": ""alice""},
                    {""type"": ""product"", ""price"": 9.99}
                ]
            }";

            using (var doc = TLDocument.FromJson(json))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var resultJson = reader.ToJson();

            using var jsonDoc = JsonDocument.Parse(resultJson);
            var root = jsonDoc.RootElement;

            Assert.True(root.TryGetProperty("data", out var data));
            Assert.Equal(JsonValueKind.Array, data.ValueKind);
            Assert.Equal(2, data.GetArrayLength());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void FromJson_ComplexNestedWithSchemaInference_CompileRoundTrips()
    {
        // Realistic scenario: uniform arrays (get schemas) alongside heterogeneous
        // nested objects (get 'any' type). Both paths must work in the same document.
        var tlbxPath = Path.Combine(_tempDir, "test_complex_inference.tlbx");

        try
        {
            const string json = @"{
                ""users"": [
                    {""name"": ""alice"", ""age"": 30, ""prefs"": {""theme"": ""dark""}},
                    {""name"": ""bob"",   ""age"": 25, ""prefs"": {""lang"": ""en"", ""tz"": ""UTC""}}
                ],
                ""config"": {""debug"": true, ""version"": 2}
            }";

            TLReader.CreateFromJson(json, tlbxPath);

            using var reader = TLReader.Open(tlbxPath);
            var resultJson = reader.ToJson();

            Assert.Contains("alice", resultJson);
            Assert.Contains("bob", resultJson);
            Assert.Contains("debug", resultJson);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void FromJson_HeterogeneousArrayInStruct_BinaryRoundTrips()
    {
        // Regression: []any fields (from JSON inference of heterogeneous arrays inside
        // schema-typed objects) caused binary corruption. encode_typed_value wrote
        // TLType::Struct as the element type for "any" (the to_tl_type default),
        // but data was heterogeneous. Reader then read garbage as schema indices.
        var tlbxPath = Path.Combine(_tempDir, "test_any_array_roundtrip.tlbx");

        try
        {
            const string json = @"{
                ""events"": [
                    {
                        ""id"": ""E1"",
                        ""type"": ""sale"",
                        ""data"": [""SKU-100"", 3, 29.99, true],
                        ""tags"": [""flash"", ""online""]
                    },
                    {
                        ""id"": ""E2"",
                        ""type"": ""return"",
                        ""data"": [""SKU-200"", 1, 15.0, false],
                        ""tags"": [""in-store""]
                    }
                ]
            }";

            using (var doc = TLDocument.FromJson(json))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var resultJson = reader.ToJson();
            using var jsonDoc = JsonDocument.Parse(resultJson);
            var root = jsonDoc.RootElement;

            Assert.True(root.TryGetProperty("events", out var events));
            Assert.Equal(JsonValueKind.Array, events.ValueKind);
            Assert.Equal(2, events.GetArrayLength());

            // Verify first event's heterogeneous data array survived binary roundtrip
            var e1 = events[0];
            Assert.Equal("E1", e1.GetProperty("id").GetString());
            var data1 = e1.GetProperty("data");
            Assert.Equal(4, data1.GetArrayLength());
            Assert.Equal("SKU-100", data1[0].GetString());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void FromJson_RetailOrdersFixture_CompileRoundTrips()
    {
        // End-to-end test with the retail_orders.json example file.
        // Exercises JSON → infer schemas → compile → binary read with complex
        // real-world data including []any fields (heterogeneous arrays inside
        // schema-typed objects like order.customer, order.payment, order.shipping).
        var jsonPath = FindRepoFile(Path.Combine("examples", "retail_orders.json"));
        if (jsonPath == null)
        {
            // Skip if examples directory not found (e.g., CI without full repo)
            return;
        }

        var tlbxPath = Path.Combine(_tempDir, "test_retail_orders.tlbx");

        try
        {
            var json = File.ReadAllText(jsonPath);

            using (var doc = TLDocument.FromJson(json))
            {
                doc.Compile(tlbxPath, compress: true);
            }

            Assert.True(File.Exists(tlbxPath));

            using var reader = TLReader.Open(tlbxPath);
            var keys = reader.Keys;
            Assert.Equal(5, keys.Length);

            // Parse the round-tripped JSON and verify data integrity
            var resultJson = reader.ToJson();
            using var jsonDoc = JsonDocument.Parse(resultJson);
            var root = jsonDoc.RootElement;

            Assert.True(root.TryGetProperty("orders", out var orders));
            Assert.Equal(10, orders.GetArrayLength());

            Assert.True(root.TryGetProperty("products", out var products));
            Assert.Equal(4, products.GetArrayLength());

            Assert.True(root.TryGetProperty("customers", out var customers));
            Assert.Equal(3, customers.GetArrayLength());

            // Spot-check: first order preserves heterogeneous fields
            var order1 = orders[0];
            Assert.Equal("ORD-2024-00001", order1.GetProperty("order_id").GetString());
            var items = order1.GetProperty("items");
            Assert.Equal(3, items.GetArrayLength());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_PreservesSpecialCharacters_NoUnicodeEscaping()
    {
        // Regression: System.Text.Json's default JavaScriptEncoder.Default HTML-encodes
        // characters like + (U+002B) as \u002B, < as \u003C, > as \u003E, etc.
        // TLReader.ToJson() must use UnsafeRelaxedJsonEscaping for data fidelity.
        var tlbxPath = Path.Combine(_tempDir, "test_special_chars.tlbx");

        try
        {
            const string json = @"{
                ""contacts"": [
                    { ""name"": ""Alice"", ""phone"": ""+1-555-123-4567"" },
                    { ""name"": ""Bob"", ""email"": ""bob@example.com"" }
                ],
                ""note"": ""x < y && y > z"",
                ""tag"": ""it's a 'test'""
            }";

            using (var doc = TLDocument.FromJson(json))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var resultJson = reader.ToJson();

            // + must not be escaped
            Assert.Contains("+1-555-123-4567", resultJson);
            Assert.DoesNotContain("\\u002B", resultJson);

            // < and > must not be escaped
            Assert.Contains("x < y", resultJson);
            Assert.DoesNotContain("\\u003C", resultJson);

            // Single quotes must not be escaped
            Assert.Contains("it's a 'test'", resultJson);
            Assert.DoesNotContain("\\u0027", resultJson);

            // Also verify compact path
            var compactJson = reader.ToJsonCompact();
            Assert.Contains("+1-555-123-4567", compactJson);
            Assert.DoesNotContain("\\u002B", compactJson);
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    [Fact]
    public void ToJson_PreservesFloatDecimalPoint_WholeNumbers()
    {
        // Regression: System.Text.Json's JsonValue.Create(double) serializes 3582.0
        // as 3582 (dropping .0). TLReader.ToJson() must preserve the decimal for
        // whole-number floats to match source JSON and Rust CLI output.
        var tlbxPath = Path.Combine(_tempDir, "test_float_decimal.tlbx");

        try
        {
            const string json = @"{
                ""products"": [
                    { ""name"": ""Widget A"", ""price"": 99.0, ""rating"": 4.5 },
                    { ""name"": ""Widget B"", ""price"": 150.0, ""rating"": 3.75 },
                    { ""name"": ""Widget C"", ""price"": 0.0, ""rating"": 5.0 }
                ]
            }";

            using (var doc = TLDocument.FromJson(json))
            {
                doc.Compile(tlbxPath);
            }

            using var reader = TLReader.Open(tlbxPath);
            var resultJson = reader.ToJson();
            using var jsonDoc = JsonDocument.Parse(resultJson);
            var root = jsonDoc.RootElement;

            var products = root.GetProperty("products");
            var p1 = products[0];
            var p2 = products[1];
            var p3 = products[2];

            // Whole-number floats must retain .0 in raw JSON text
            Assert.Contains("99.0", resultJson);
            Assert.Contains("150.0", resultJson);
            Assert.Contains("0.0", resultJson);
            Assert.Contains("5.0", resultJson);

            // Non-whole floats must preserve their decimal digits
            Assert.Equal(4.5, p1.GetProperty("rating").GetDouble());
            Assert.Equal(3.75, p2.GetProperty("rating").GetDouble());

            // Values must still parse as correct doubles
            Assert.Equal(99.0, p1.GetProperty("price").GetDouble());
            Assert.Equal(150.0, p2.GetProperty("price").GetDouble());
            Assert.Equal(0.0, p3.GetProperty("price").GetDouble());
        }
        finally
        {
            if (File.Exists(tlbxPath))
                File.Delete(tlbxPath);
        }
    }

    // ==========================================================================
    // Fixture-Based Tests (for types that can't be created from text)
    // ==========================================================================
    // These fixtures are created by: cargo run --example create_test_fixtures

    private static string GetFixturePath(string filename)
    {
        // Try multiple paths to find fixtures (works from different run locations)
        var candidates = new[]
        {
            Path.Combine("fixtures", filename),
            Path.Combine("..", "..", "..", "fixtures", filename),
            Path.Combine(AppContext.BaseDirectory, "fixtures", filename),
        };

        foreach (var path in candidates)
        {
            if (File.Exists(path))
                return path;
        }

        // Return the relative path - test will skip if not found
        return Path.Combine("fixtures", filename);
    }

    [Fact]
    public void Bytes_FromFixture_ReturnsCorrectData()
    {
        var fixturePath = GetFixturePath("bytes_test.tlbx");
        if (!File.Exists(fixturePath))
        {
            // Skip if fixture not available (run: cargo run --example create_test_fixtures)
            return;
        }

        using var reader = TLReader.Open(fixturePath);

        // Test non-empty bytes
        using var binaryData = reader["binary_data"];
        Assert.NotNull(binaryData);
        Assert.Equal(TLType.Bytes, binaryData.Type);

        var bytes = binaryData.AsBytes();
        Assert.NotNull(bytes);
        Assert.Equal(4, bytes.Length);
        Assert.Equal(0xde, bytes[0]);
        Assert.Equal(0xad, bytes[1]);
        Assert.Equal(0xbe, bytes[2]);
        Assert.Equal(0xef, bytes[3]);

        // Test empty bytes
        using var emptyBytes = reader["empty_bytes"];
        Assert.NotNull(emptyBytes);
        Assert.Equal(TLType.Bytes, emptyBytes.Type);
        var empty = emptyBytes.AsBytes();
        Assert.NotNull(empty);
        Assert.Empty(empty);
    }

    [Fact]
    public void ToJson_Bytes_HasCorrectHexFormat()
    {
        var fixturePath = GetFixturePath("bytes_test.tlbx");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = TLReader.Open(fixturePath);
        var json = reader.ToJson();

        // Parse and verify structure
        using var jsonDoc = JsonDocument.Parse(json);
        var root = jsonDoc.RootElement;

        // binary_data should be "0xcafef00d"
        Assert.True(root.TryGetProperty("binary_data", out var binaryData));
        Assert.Equal(JsonValueKind.String, binaryData.ValueKind);
        Assert.Equal("0xcafef00d", binaryData.GetString());

        // empty_bytes should be "0x"
        Assert.True(root.TryGetProperty("empty_bytes", out var emptyBytes));
        Assert.Equal(JsonValueKind.String, emptyBytes.ValueKind);
        Assert.Equal("0x", emptyBytes.GetString());
    }

    [Fact]
    public void Timestamp_FromFixture_ReturnsCorrectData()
    {
        var fixturePath = GetFixturePath("timestamp_test.tlbx");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = TLReader.Open(fixturePath);

        // Test specific timestamp
        using var created = reader["created"];
        Assert.NotNull(created);
        Assert.Equal(TLType.Timestamp, created.Type);
        Assert.Equal(1705315800000, created.AsTimestamp());

        var dt = created.AsDateTime();
        Assert.NotNull(dt);
        Assert.Equal(2024, dt.Value.Year);
        Assert.Equal(1, dt.Value.Month);
        Assert.Equal(15, dt.Value.Day);

        // Test epoch
        using var epoch = reader["epoch"];
        Assert.NotNull(epoch);
        Assert.Equal(0, epoch.AsTimestamp());
    }

    [Fact]
    public void ToJson_Timestamp_HasIso8601Format()
    {
        var fixturePath = GetFixturePath("timestamp_test.tlbx");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = TLReader.Open(fixturePath);
        var json = reader.ToJson();

        // Parse and verify structure
        using var jsonDoc = JsonDocument.Parse(json);
        var root = jsonDoc.RootElement;

        // created should be ISO 8601 format
        Assert.True(root.TryGetProperty("created", out var created));
        Assert.Equal(JsonValueKind.String, created.ValueKind);
        var createdStr = created.GetString();
        Assert.NotNull(createdStr);
        Assert.Contains("2024-01-15", createdStr);
        Assert.Contains("10:30:00", createdStr);

        // epoch should be 1970-01-01
        Assert.True(root.TryGetProperty("epoch", out var epoch));
        Assert.Equal(JsonValueKind.String, epoch.ValueKind);
        var epochStr = epoch.GetString();
        Assert.NotNull(epochStr);
        Assert.Contains("1970-01-01", epochStr);
    }

    // ==========================================================================
    // Cross-Language Parity Test
    // ==========================================================================
    // This test ensures .NET produces the same JSON as Rust CLI for all types.
    // The comprehensive.tlbx fixture contains ALL special types.

    [Fact]
    public void CrossLanguageParity_AllTypes_MatchExpectedJson()
    {
        var fixturePath = GetFixturePath("comprehensive.tlbx");
        var expectedPath = GetFixturePath("comprehensive.expected.json");

        if (!File.Exists(fixturePath) || !File.Exists(expectedPath))
        {
            // Skip if fixtures not available
            // Run: cargo run --example create_test_fixtures
            return;
        }

        using var reader = TLReader.Open(fixturePath);
        var actualJson = reader.ToJson();

        // Parse both JSONs for structural comparison
        using var actualDoc = JsonDocument.Parse(actualJson);
        var expectedText = File.ReadAllText(expectedPath);
        using var expectedDoc = JsonDocument.Parse(expectedText);

        var actual = actualDoc.RootElement;
        var expected = expectedDoc.RootElement;

        // Verify each expected key exists and has correct type/value
        // Primitives
        VerifyJsonValue(actual, expected, "null_val", JsonValueKind.Null);
        VerifyJsonValue(actual, expected, "bool_true", JsonValueKind.True);
        VerifyJsonValue(actual, expected, "bool_false", JsonValueKind.False);
        VerifyJsonNumber(actual, expected, "int_val", 42);
        VerifyJsonNumber(actual, expected, "int_neg", -123);
        VerifyJsonNumber(actual, expected, "uint_val", 999);
        VerifyJsonString(actual, expected, "string_val", "hello world");

        // Float (check approximately)
        Assert.True(actual.TryGetProperty("float_val", out var floatVal));
        Assert.Equal(JsonValueKind.Number, floatVal.ValueKind);
        Assert.Equal(3.14159, floatVal.GetDouble(), 4);

        // Bytes - verify hex format
        VerifyJsonString(actual, expected, "bytes_val", "0xcafebabe");
        VerifyJsonString(actual, expected, "bytes_empty", "0x");

        // Timestamp - verify ISO 8601 contains date
        Assert.True(actual.TryGetProperty("timestamp_val", out var ts));
        Assert.Equal(JsonValueKind.String, ts.ValueKind);
        Assert.Contains("2024-01-15", ts.GetString());

        // Array
        Assert.True(actual.TryGetProperty("array_val", out var arr));
        Assert.Equal(JsonValueKind.Array, arr.ValueKind);
        Assert.Equal(3, arr.GetArrayLength());

        // Object
        Assert.True(actual.TryGetProperty("object_val", out var obj));
        Assert.Equal(JsonValueKind.Object, obj.ValueKind);
        Assert.True(obj.TryGetProperty("name", out _));
        Assert.True(obj.TryGetProperty("age", out _));

        // Ref - verify $ref structure
        Assert.True(actual.TryGetProperty("ref_val", out var refVal));
        Assert.Equal(JsonValueKind.Object, refVal.ValueKind);
        Assert.True(refVal.TryGetProperty("$ref", out var refName));
        Assert.Equal("object_val", refName.GetString());

        // Tagged - verify $tag/$value structure
        Assert.True(actual.TryGetProperty("tagged_val", out var taggedVal));
        Assert.Equal(JsonValueKind.Object, taggedVal.ValueKind);
        Assert.True(taggedVal.TryGetProperty("$tag", out var tagName));
        Assert.Equal("ok", tagName.GetString());
        Assert.True(taggedVal.TryGetProperty("$value", out var tagValue));
        Assert.Equal(200, tagValue.GetInt64());

        // Map - verify array of pairs structure
        Assert.True(actual.TryGetProperty("map_val", out var mapVal));
        Assert.Equal(JsonValueKind.Array, mapVal.ValueKind);
        Assert.Equal(2, mapVal.GetArrayLength());
        var pair0 = mapVal[0];
        Assert.Equal(JsonValueKind.Array, pair0.ValueKind);
        Assert.Equal(2, pair0.GetArrayLength());
    }

    /// <summary>
    /// Walk up from AppContext.BaseDirectory until we find a directory containing
    /// the given relative path (e.g. "examples/retail_orders.json").
    /// Returns null if not found (caller should skip the test).
    /// </summary>
    private static string? FindRepoFile(string relativePath)
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null)
        {
            var candidate = Path.Combine(dir.FullName, relativePath);
            if (File.Exists(candidate))
                return candidate;
            dir = dir.Parent;
        }
        return null;
    }

    private static void VerifyJsonValue(JsonElement actual, JsonElement expected, string key, JsonValueKind expectedKind)
    {
        Assert.True(actual.TryGetProperty(key, out var actualVal), $"Missing key: {key}");
        Assert.Equal(expectedKind, actualVal.ValueKind);
    }

    private static void VerifyJsonNumber(JsonElement actual, JsonElement expected, string key, long expectedValue)
    {
        Assert.True(actual.TryGetProperty(key, out var actualVal), $"Missing key: {key}");
        Assert.Equal(JsonValueKind.Number, actualVal.ValueKind);
        Assert.Equal(expectedValue, actualVal.GetInt64());
    }

    private static void VerifyJsonString(JsonElement actual, JsonElement expected, string key, string expectedValue)
    {
        Assert.True(actual.TryGetProperty(key, out var actualVal), $"Missing key: {key}");
        Assert.Equal(JsonValueKind.String, actualVal.ValueKind);
        Assert.Equal(expectedValue, actualVal.GetString());
    }
}
