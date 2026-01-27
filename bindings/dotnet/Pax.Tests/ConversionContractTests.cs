using System.Text.Json;
using Xunit;

namespace Pax.Tests;

/// <summary>
/// Conversion contract tests that lock down JSON↔PAX behavior.
///
/// STABILITY POLICY:
/// - Plain JSON roundtrip: MUST be lossless for primitives, arrays, objects
/// - PAX→JSON: Special types have FIXED representations that MUST NOT change
/// - JSON→PAX: No magic parsing; $ref/$tag/hex/ISO8601 stay as plain JSON
/// </summary>
public class ConversionContractTests
{
    // =========================================================================
    // Plain JSON Roundtrip (STABLE)
    // =========================================================================

    [Theory]
    [InlineData("null")]
    [InlineData("true")]
    [InlineData("false")]
    [InlineData("0")]
    [InlineData("42")]
    [InlineData("-123")]
    [InlineData("3.14")]
    [InlineData("\"\"")]
    [InlineData("\"hello world\"")]
    [InlineData("\"日本語\"")]
    [InlineData("[]")]
    [InlineData("[1, 2, 3]")]
    [InlineData("[1, \"two\", true, null]")]
    [InlineData("[[1, 2], [3, 4]]")]
    public void Contract_PlainJson_Roundtrip(string json)
    {
        // Wrap in object for top-level non-objects
        var wrappedJson = json.StartsWith("{") ? json : $"{{\"v\": {json}}}";

        using var doc = PaxDocument.FromJson(wrappedJson);
        Assert.NotNull(doc);

        var outputJson = doc.ToJsonCompact();
        Assert.NotNull(outputJson);

        // Parse both and compare semantically (whitespace doesn't matter)
        using var inputParsed = JsonDocument.Parse(wrappedJson);
        using var outputParsed = JsonDocument.Parse(outputJson);

        // Verify the value is preserved by comparing normalized JSON
        if (!json.StartsWith("{"))
        {
            var inputValue = inputParsed.RootElement.GetProperty("v");
            var outputValue = outputParsed.RootElement.GetProperty("v");
            // Compare by re-serializing both with same options (compact, no whitespace)
            var inputNormalized = JsonSerializer.Serialize(inputValue);
            var outputNormalized = JsonSerializer.Serialize(outputValue);
            Assert.Equal(inputNormalized, outputNormalized);
        }
    }

    [Fact]
    public void Contract_Object_KeysPreserved()
    {
        const string json = @"{""name"": ""alice"", ""age"": 30, ""active"": true}";

        using var doc = PaxDocument.FromJson(json);
        var outputJson = doc.ToJsonCompact();

        var output = JsonDocument.Parse(outputJson);
        Assert.True(output.RootElement.TryGetProperty("name", out var name));
        Assert.Equal("alice", name.GetString());
        Assert.True(output.RootElement.TryGetProperty("age", out var age));
        Assert.Equal(30, age.GetInt64());
        Assert.True(output.RootElement.TryGetProperty("active", out var active));
        Assert.True(active.GetBoolean());
    }

    [Fact]
    public void Contract_NestedObject_Preserved()
    {
        const string json = @"{""a"": {""b"": {""c"": {""d"": 5}}}}";

        using var doc = PaxDocument.FromJson(json);

        using var a = doc["a"];
        Assert.Equal(PaxType.Object, a?.Type);

        using var b = a?["b"];
        Assert.Equal(PaxType.Object, b?.Type);

        using var c = b?["c"];
        Assert.Equal(PaxType.Object, c?.Type);

        using var d = c?["d"];
        Assert.Equal(5, d?.AsInt());
    }

    // =========================================================================
    // JSON→PAX No Magic (STABLE)
    // These tests ensure JSON special patterns are NOT auto-converted
    // =========================================================================

    [Fact]
    public void Contract_JsonDollarRef_StaysObject()
    {
        // CONTRACT: JSON {"$ref": ...} MUST remain an Object, NOT become Ref
        const string json = @"{""x"": {""$ref"": ""some_key""}}";

        using var doc = PaxDocument.FromJson(json);
        using var x = doc["x"];

        // Must be Object type, not Ref
        Assert.Equal(PaxType.Object, x?.Type);
        Assert.Null(x?.AsRefName()); // Should NOT be a Ref

        // Should have $ref as a key in the object
        var keys = x?.GetObjectKeys();
        Assert.NotNull(keys);
        Assert.Contains("$ref", keys);
    }

    [Fact]
    public void Contract_JsonDollarTag_StaysObject()
    {
        // CONTRACT: JSON {"$tag": ..., "$value": ...} MUST remain an Object
        const string json = @"{""x"": {""$tag"": ""ok"", ""$value"": 200}}";

        using var doc = PaxDocument.FromJson(json);
        using var x = doc["x"];

        // Must be Object type, not Tagged
        Assert.Equal(PaxType.Object, x?.Type);
        Assert.Null(x?.AsTagName()); // Should NOT be a Tagged

        // Should have $tag and $value as keys
        var keys = x?.GetObjectKeys();
        Assert.NotNull(keys);
        Assert.Contains("$tag", keys);
        Assert.Contains("$value", keys);
    }

    [Fact]
    public void Contract_JsonHexString_StaysString()
    {
        // CONTRACT: Hex strings MUST remain String, NOT become Bytes
        const string json = @"{""x"": ""0xdeadbeef""}";

        using var doc = PaxDocument.FromJson(json);
        using var x = doc["x"];

        Assert.Equal(PaxType.String, x?.Type);
        Assert.Equal("0xdeadbeef", x?.AsString());
        Assert.Null(x?.AsBytes()); // Should NOT be Bytes
    }

    [Fact]
    public void Contract_JsonIsoTimestamp_StaysString()
    {
        // CONTRACT: ISO 8601 strings MUST remain String, NOT become Timestamp
        const string json = @"{""x"": ""2024-01-15T10:30:00.000Z""}";

        using var doc = PaxDocument.FromJson(json);
        using var x = doc["x"];

        Assert.Equal(PaxType.String, x?.Type);
        Assert.Equal("2024-01-15T10:30:00.000Z", x?.AsString());
        Assert.Null(x?.AsTimestamp()); // Should NOT be Timestamp
    }

    [Fact]
    public void Contract_JsonArrayPairs_StaysArray()
    {
        // CONTRACT: Array of pairs MUST remain Array, NOT become Map
        const string json = @"{""x"": [[1, ""one""], [2, ""two""]]}";

        using var doc = PaxDocument.FromJson(json);
        using var x = doc["x"];

        Assert.Equal(PaxType.Array, x?.Type);
        Assert.Equal(2, x?.ArrayLength);

        // Verify it's not a Map
        Assert.Equal(0, x?.MapLength);
    }

    // =========================================================================
    // Number Type Inference (STABLE)
    // =========================================================================

    [Fact]
    public void Contract_JsonInteger_BecomesInt()
    {
        const string json = @"{""n"": 42}";

        using var doc = PaxDocument.FromJson(json);
        using var n = doc["n"];

        // CONTRACT: Integers that fit i64 become Int
        Assert.Equal(PaxType.Int, n?.Type);
        Assert.Equal(42, n?.AsInt());
    }

    [Fact]
    public void Contract_JsonNegativeInteger_BecomesInt()
    {
        const string json = @"{""n"": -9223372036854775808}";

        using var doc = PaxDocument.FromJson(json);
        using var n = doc["n"];

        // CONTRACT: Negative integers become Int
        Assert.Equal(PaxType.Int, n?.Type);
    }

    [Fact]
    public void Contract_JsonFloat_BecomesFloat()
    {
        const string json = @"{""n"": 3.14159}";

        using var doc = PaxDocument.FromJson(json);
        using var n = doc["n"];

        // CONTRACT: Numbers with decimals become Float
        Assert.Equal(PaxType.Float, n?.Type);
        var value = n?.AsFloat();
        Assert.NotNull(value);
        Assert.True(Math.Abs(3.14159 - value.Value) < 0.00001);
    }

    // =========================================================================
    // PAX→JSON Fixed Representations (via Binary)
    // These tests verify the JSON output format from binary files
    // =========================================================================

    [Fact]
    public void Contract_Bytes_ToJson_HexFormat()
    {
        // Use the fixture file which has Bytes values
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "fixtures", "bytes_test.paxb");

        if (!File.Exists(fixturePath))
        {
            // Skip if fixture not available
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
        var json = reader.ToJson();

        // CONTRACT: Bytes serialize as lowercase hex with 0x prefix
        Assert.Contains("0x", json);
    }

    [Fact]
    public void Contract_Ref_ToJson_Format()
    {
        // Use the comprehensive fixture
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "fixtures", "comprehensive.paxb");

        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
        var json = reader.ToJson();

        // CONTRACT: Ref serializes as {"$ref": "name"}
        Assert.Contains("\"$ref\"", json);
    }

    [Fact]
    public void Contract_Tagged_ToJson_Format()
    {
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "fixtures", "comprehensive.paxb");

        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
        var json = reader.ToJson();

        // CONTRACT: Tagged serializes with $tag and $value keys
        Assert.Contains("\"$tag\"", json);
        Assert.Contains("\"$value\"", json);
    }

    [Fact]
    public void Contract_Map_ToJson_ArrayPairs()
    {
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "fixtures", "comprehensive.paxb");

        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
        var json = reader.ToJsonCompact();

        // CONTRACT: Map serializes as array of [key, value] pairs
        // The map_val key should have nested arrays
        Assert.Contains("[[", json);
    }

    [Fact]
    public void Contract_Timestamp_ToJson_Iso8601()
    {
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "fixtures", "timestamp_test.paxb");

        if (!File.Exists(fixturePath))
        {
            return;
        }

        using var reader = PaxReader.Open(fixturePath);
        var json = reader.ToJson();

        // CONTRACT: Timestamp serializes as ISO 8601
        // Should contain a date pattern
        Assert.Matches(@"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", json);
    }

    // =========================================================================
    // Edge Cases (STABLE)
    // =========================================================================

    [Fact]
    public void Contract_EmptyObject_Preserved()
    {
        const string json = @"{""empty"": {}}";

        using var doc = PaxDocument.FromJson(json);
        using var empty = doc["empty"];

        Assert.Equal(PaxType.Object, empty?.Type);
        var keys = empty?.GetObjectKeys();
        Assert.Empty(keys ?? Array.Empty<string>());
    }

    [Fact]
    public void Contract_EmptyArray_Preserved()
    {
        const string json = @"{""empty"": []}";

        using var doc = PaxDocument.FromJson(json);
        using var empty = doc["empty"];

        Assert.Equal(PaxType.Array, empty?.Type);
        Assert.Equal(0, empty?.ArrayLength);
    }

    [Fact]
    public void Contract_UnicodeStrings_Preserved()
    {
        const string json = @"{""greeting"": ""こんにちは世界"", ""emoji"": ""🎉🚀""}";

        using var doc = PaxDocument.FromJson(json);

        using var greeting = doc["greeting"];
        Assert.Equal("こんにちは世界", greeting?.AsString());

        using var emoji = doc["emoji"];
        Assert.Equal("🎉🚀", emoji?.AsString());
    }

    [Fact]
    public void Contract_SpecialCharacters_Preserved()
    {
        const string json = @"{""text"": ""line1\nline2\ttab\\backslash\"" quote""}";

        using var doc = PaxDocument.FromJson(json);
        using var text = doc["text"];

        Assert.Contains("\n", text?.AsString());
        Assert.Contains("\t", text?.AsString());
        Assert.Contains("\\", text?.AsString());
        Assert.Contains("\"", text?.AsString());
    }

    [Fact]
    public void Contract_LargeArray_NotTruncated()
    {
        // Build a large array
        var items = string.Join(", ", Enumerable.Range(0, 1000));
        var json = $"{{\"arr\": [{items}]}}";

        using var doc = PaxDocument.FromJson(json);
        using var arr = doc["arr"];

        // CONTRACT: Large arrays MUST be handled without truncation
        Assert.Equal(1000, arr?.ArrayLength);

        // Verify first and last elements
        using var first = arr?[0];
        Assert.Equal(0, first?.AsInt());

        using var last = arr?[999];
        Assert.Equal(999, last?.AsInt());
    }

    [Fact]
    public void Contract_NullValue_Preserved()
    {
        const string json = @"{""value"": null}";

        using var doc = PaxDocument.FromJson(json);
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Equal(PaxType.Null, value.Type);
    }

    [Fact]
    public void Contract_MixedArray_TypesPreserved()
    {
        const string json = @"{""mixed"": [1, ""two"", true, null, 3.14, [1, 2], {""a"": 1}]}";

        using var doc = PaxDocument.FromJson(json);
        using var mixed = doc["mixed"];

        Assert.Equal(7, mixed?.ArrayLength);

        using var e0 = mixed?[0];
        Assert.Equal(PaxType.Int, e0?.Type);

        using var e1 = mixed?[1];
        Assert.Equal(PaxType.String, e1?.Type);

        using var e2 = mixed?[2];
        Assert.Equal(PaxType.Bool, e2?.Type);

        using var e3 = mixed?[3];
        Assert.Equal(PaxType.Null, e3?.Type);

        using var e4 = mixed?[4];
        Assert.Equal(PaxType.Float, e4?.Type);

        using var e5 = mixed?[5];
        Assert.Equal(PaxType.Array, e5?.Type);

        using var e6 = mixed?[6];
        Assert.Equal(PaxType.Object, e6?.Type);
    }
}
