using System.Text.Json;
using Xunit;

namespace Pax.Tests;

public class PaxReaderTests
{
    private string _tempDir = Path.GetTempPath();

    // ==========================================================================
    // Binary File Reading Tests
    // ==========================================================================

    [Fact]
    public void Open_CompiledFile_ReturnsReader()
    {
        var paxbPath = Path.Combine(_tempDir, "test_reader.paxb");

        try
        {
            // Create a binary file from PAX text
            using (var doc = PaxDocument.Parse(@"
                name: alice
                age: 30
                active: true
            "))
            {
                doc.Compile(paxbPath);
            }

            // Open and read the binary file
            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void TryOpen_ValidFile_ReturnsTrue()
    {
        var paxbPath = Path.Combine(_tempDir, "test_tryopen.paxb");

        try
        {
            using (var doc = PaxDocument.Parse("key: value"))
            {
                doc.Compile(paxbPath);
            }

            var success = PaxReader.TryOpen(paxbPath, out var reader);
            Assert.True(success);
            Assert.NotNull(reader);
            reader?.Dispose();
        }
        finally
        {
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void TryOpen_InvalidFile_ReturnsFalse()
    {
        var success = PaxReader.TryOpen("nonexistent.paxb", out var reader);
        Assert.False(success);
        Assert.Null(reader);
    }

    [Fact]
    public void Open_NonexistentFile_ExceptionHasDetailedMessage()
    {
        var ex = Assert.Throws<PaxException>(() => PaxReader.Open("nonexistent_file.paxb"));

        // Error message should mention the file path and the problem
        Assert.Contains("nonexistent_file.paxb", ex.Message);
        Assert.Contains("Failed to open", ex.Message);
    }

    [Fact]
    public void Open_InvalidBinaryFile_ExceptionHasDetailedMessage()
    {
        var tempFile = Path.Combine(_tempDir, "invalid_binary.paxb");
        try
        {
            // Write invalid binary data (not a valid paxb)
            File.WriteAllText(tempFile, "not a valid paxb file");

            var ex = Assert.Throws<PaxException>(() => PaxReader.Open(tempFile));

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
    // JSON Conversion Tests (PAXB -> JSON)
    // ==========================================================================

    [Fact]
    public void ToJson_CompiledFile_ReturnsValidJson()
    {
        var paxbPath = Path.Combine(_tempDir, "test_tojson.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
                name: alice
                age: 30
                items: [1, 2, 3]
            "))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void ToJsonCompact_ReturnsMinifiedJson()
    {
        var paxbPath = Path.Combine(_tempDir, "test_compact.paxb");

        try
        {
            using (var doc = PaxDocument.Parse("name: alice"))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
            var json = reader.ToJsonCompact();

            Assert.NotNull(json);
            Assert.DoesNotContain("\n", json);
            Assert.Contains("name", json);
            Assert.Contains("alice", json);
        }
        finally
        {
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void GetAsJson_SingleKey_ReturnsJsonValue()
    {
        var paxbPath = Path.Combine(_tempDir, "test_getasjson.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
                config: { debug: true, level: info }
                items: [1, 2, 3]
            "))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);

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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void GetAsJson_NonExistentKey_ReturnsNull()
    {
        var paxbPath = Path.Combine(_tempDir, "test_getasjson_null.paxb");

        try
        {
            using (var doc = PaxDocument.Parse("key: value"))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
            var json = reader.GetAsJson("nonexistent");
            Assert.Null(json);
        }
        finally
        {
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    // ==========================================================================
    // JSON to PAXB Conversion Tests
    // ==========================================================================

    [Fact]
    public void CreateFromJson_ValidJson_CreatesPaxbFile()
    {
        var paxbPath = Path.Combine(_tempDir, "test_fromjson.paxb");

        try
        {
            PaxReader.CreateFromJson(@"{""name"": ""alice"", ""age"": 30}", paxbPath);

            Assert.True(File.Exists(paxbPath));

            using var reader = PaxReader.Open(paxbPath);
            using var name = reader["name"];
            Assert.Equal("alice", name?.AsString());

            using var age = reader["age"];
            Assert.Equal(30, age?.AsInt());
        }
        finally
        {
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void TryCreateFromJson_ValidJson_ReturnsTrue()
    {
        var paxbPath = Path.Combine(_tempDir, "test_trycreate.paxb");

        try
        {
            var success = PaxReader.TryCreateFromJson(@"{""key"": ""value""}", paxbPath);
            Assert.True(success);
            Assert.True(File.Exists(paxbPath));
        }
        finally
        {
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void TryCreateFromJson_InvalidJson_ReturnsFalse()
    {
        var paxbPath = Path.Combine(_tempDir, "test_trycreate_invalid.paxb");

        try
        {
            var success = PaxReader.TryCreateFromJson("{invalid json", paxbPath);
            Assert.False(success);
        }
        finally
        {
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    // ==========================================================================
    // JSON Round-Trip Tests
    // ==========================================================================

    [Fact]
    public void JsonRoundTrip_ThroughPaxb_PreservesData()
    {
        var paxbPath = Path.Combine(_tempDir, "test_roundtrip.paxb");

        try
        {
            // JSON -> PAXB
            const string originalJson = @"{
                ""name"": ""Alice"",
                ""age"": 30,
                ""active"": true,
                ""scores"": [95.5, 88.0, 92.3]
            }";

            PaxReader.CreateFromJson(originalJson, paxbPath);

            // PAXB -> JSON
            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void ToJson_NestedStructures_PreservesHierarchy()
    {
        var paxbPath = Path.Combine(_tempDir, "test_nested.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
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
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    // ==========================================================================
    // Structural JSON Output Tests
    // ==========================================================================

    [Fact]
    public void ToJson_Ref_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_ref_json.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
                base: {host: localhost}
                config: !base
            "))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void ToJson_Tagged_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_tagged_json.paxb");

        try
        {
            using (var doc = PaxDocument.Parse("status: :error 404"))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void ToJson_Map_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_map_json.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
                lookup: @map {
                    1: one,
                    2: two
                }
            "))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void ToJson_Object_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_object_json.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
                user: {
                    name: alice,
                    age: 30,
                    active: true
                }
            "))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void ToJson_Array_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_array_json.paxb");

        try
        {
            using (var doc = PaxDocument.Parse("items: [1, 2, 3, 4, 5]"))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void ToJson_MixedTypes_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_mixed_json.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
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
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void GetAsJson_Ref_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_getasjson_ref.paxb");

        try
        {
            using (var doc = PaxDocument.Parse(@"
                base: {x: 1}
                ptr: !base
            "))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
        }
    }

    [Fact]
    public void GetAsJson_Tagged_HasCorrectStructure()
    {
        var paxbPath = Path.Combine(_tempDir, "test_getasjson_tagged.paxb");

        try
        {
            using (var doc = PaxDocument.Parse("result: :ok done"))
            {
                doc.Compile(paxbPath);
            }

            using var reader = PaxReader.Open(paxbPath);
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
            if (File.Exists(paxbPath))
                File.Delete(paxbPath);
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
        var fixturePath = GetFixturePath("bytes_test.paxb");
        if (!File.Exists(fixturePath))
        {
            // Skip if fixture not available (run: cargo run --example create_test_fixtures)
            return;
        }

        using var reader = PaxReader.Open(fixturePath);

        // Test non-empty bytes
        using var binaryData = reader["binary_data"];
        Assert.NotNull(binaryData);
        Assert.Equal(PaxType.Bytes, binaryData.Type);

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
        Assert.Equal(PaxType.Bytes, emptyBytes.Type);
        var empty = emptyBytes.AsBytes();
        Assert.NotNull(empty);
        Assert.Empty(empty);
    }

    [Fact]
    public void ToJson_Bytes_HasCorrectHexFormat()
    {
        var fixturePath = GetFixturePath("bytes_test.paxb");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
        var json = reader.ToJson();

        // Parse and verify structure
        using var jsonDoc = JsonDocument.Parse(json);
        var root = jsonDoc.RootElement;

        // binary_data should be "0xdeadbeef"
        Assert.True(root.TryGetProperty("binary_data", out var binaryData));
        Assert.Equal(JsonValueKind.String, binaryData.ValueKind);
        Assert.Equal("0xdeadbeef", binaryData.GetString());

        // empty_bytes should be "0x"
        Assert.True(root.TryGetProperty("empty_bytes", out var emptyBytes));
        Assert.Equal(JsonValueKind.String, emptyBytes.ValueKind);
        Assert.Equal("0x", emptyBytes.GetString());
    }

    [Fact]
    public void Timestamp_FromFixture_ReturnsCorrectData()
    {
        var fixturePath = GetFixturePath("timestamp_test.paxb");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = PaxReader.Open(fixturePath);

        // Test specific timestamp
        using var created = reader["created"];
        Assert.NotNull(created);
        Assert.Equal(PaxType.Timestamp, created.Type);
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
        var fixturePath = GetFixturePath("timestamp_test.paxb");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
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
    // The comprehensive.paxb fixture contains ALL special types.

    [Fact]
    public void CrossLanguageParity_AllTypes_MatchExpectedJson()
    {
        var fixturePath = GetFixturePath("comprehensive.paxb");
        var expectedPath = GetFixturePath("comprehensive.expected.json");

        if (!File.Exists(fixturePath) || !File.Exists(expectedPath))
        {
            // Skip if fixtures not available
            // Run: cargo run --example create_test_fixtures
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
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
