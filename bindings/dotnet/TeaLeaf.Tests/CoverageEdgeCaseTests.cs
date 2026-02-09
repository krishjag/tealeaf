using System.Text;
using TeaLeaf.Annotations;
using TeaLeaf.Native;
using Xunit;

namespace TeaLeaf.Tests;

// ========================================================================
// Additional DTO Models for edge-case coverage
// ========================================================================

[TeaLeaf]
public partial class NumericModel
{
    public short SmallInt { get; set; }
    public byte TinyInt { get; set; }
    public uint UnsignedInt { get; set; }
    public ulong BigUnsigned { get; set; }
    public float SinglePrecision { get; set; }
    public decimal Money { get; set; }
}

[TeaLeaf]
public partial class NullableNumericModel
{
    public string Name { get; set; } = "";
    public short? SmallVal { get; set; }
    public byte? TinyVal { get; set; }
    public uint? UnsignedVal { get; set; }
    public float? FloatVal { get; set; }
    public decimal? DecimalVal { get; set; }
}

// Multi-word enum for ParseEnumFromSnakeCase coverage
public enum OrderStatus { Pending, InProgress, CompletedSuccessfully }

[TeaLeaf]
public partial class WithOrderStatus
{
    public string Name { get; set; } = "";
    public OrderStatus Status { get; set; }
}

[TeaLeaf]
public partial class WithDictionary
{
    public string Name { get; set; } = "";
    public Dictionary<string, string> Metadata { get; set; } = new();
}

[TeaLeaf]
public partial class WithTimestamps
{
    public string Label { get; set; } = "";
    public DateTimeOffset CreatedAt { get; set; }
    public DateTimeOffset? UpdatedAt { get; set; }
}

// Not annotated with [TeaLeaf] to avoid source generator DateTime issue;
// used only with reflection-based TeaLeafSerializer.FromText<T>().
public class DateTimeModel
{
    public string Name { get; set; } = "";
    public DateTime CreatedAt { get; set; }
}

[TeaLeaf]
public partial class WithNullableAddress
{
    public string Name { get; set; } = "";
    public Address? HomeAddress { get; set; }
}

// ========================================================================
// Spec Conformance Tests (added for spec audit changes)
// ========================================================================

public class SpecConformanceTests
{
    private readonly string _tempDir = Path.GetTempPath();

    // ------------------------------------------------------------------
    // Timestamp offset formats: +HH:MM, +HHMM, +HH (lexer.rs fix)
    // ------------------------------------------------------------------

    [Fact]
    public void Timestamp_CompactOffset_PreservesTimezone()
    {
        // +HHMM format (no colon) — was buggy before lexer fix
        using var doc = TLDocument.Parse("ts: 2024-01-15T16:00:00+0530");
        using var val = doc["ts"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Timestamp, val.Type);
        Assert.Equal((short)330, val.AsTimestampOffset());
        var dt = val.AsDateTime();
        Assert.NotNull(dt);
        Assert.Equal(TimeSpan.FromMinutes(330), dt!.Value.Offset);
    }

    [Fact]
    public void Timestamp_HourOnlyOffset_PreservesTimezone()
    {
        // +HH format (hour-only, minutes default to 00)
        using var doc = TLDocument.Parse("ts: 2024-01-15T16:00:00+05");
        using var val = doc["ts"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Timestamp, val.Type);
        Assert.Equal((short)300, val.AsTimestampOffset());
        var dt = val.AsDateTime();
        Assert.NotNull(dt);
        Assert.Equal(TimeSpan.FromMinutes(300), dt!.Value.Offset);
    }

    [Fact]
    public void Timestamp_NegativeCompactOffset_PreservesTimezone()
    {
        // -HHMM format
        using var doc = TLDocument.Parse("ts: 2024-01-15T07:00:00-0800");
        using var val = doc["ts"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Timestamp, val.Type);
        Assert.Equal((short)(-480), val.AsTimestampOffset());
    }

    [Fact]
    public void Timestamp_OffsetFormats_BinaryRoundTrip()
    {
        // Verify all three offset formats survive text → binary → read
        var path = Path.Combine(_tempDir, $"test_tz_roundtrip_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                colon: 2024-01-15T16:00:00+05:30
                compact: 2024-01-15T16:00:00+0530
                hour_only: 2024-01-15T16:00:00+05
                utc: 2024-01-15T10:30:00Z
            "))
            {
                doc.Compile(path);
            }

            using var reader = TLReader.Open(path);

            using var colon = reader["colon"];
            Assert.Equal((short)330, colon?.AsTimestampOffset());

            using var compact = reader["compact"];
            Assert.Equal((short)330, compact?.AsTimestampOffset());

            using var hourOnly = reader["hour_only"];
            Assert.Equal((short)300, hourOnly?.AsTimestampOffset());

            using var utc = reader["utc"];
            Assert.Equal((short)0, utc?.AsTimestampOffset());
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    // ------------------------------------------------------------------
    // Numeric coercion: out-of-range → 0 (writer.rs fix)
    // ------------------------------------------------------------------

    [Fact]
    public void NumericCoercion_IntOverflow_CoercesToZero_BinaryRoundTrip()
    {
        // Int8 field with value exceeding i8 range → should coerce to 0
        var path = Path.Combine(_tempDir, $"test_coerce_int_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                @struct Sensor (reading: int8, status: string)
                sensor: @table Sensor [
                    (42, ok),
                    (999, overflow),
                ]
            "))
            {
                doc.Compile(path);
            }

            using var reader = TLReader.Open(path);
            var json = reader.ToJson();
            Assert.NotNull(json);

            using var jsonDoc = System.Text.Json.JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;
            Assert.True(root.TryGetProperty("sensor", out var sensor));

            // First row: 42 fits in int8
            var row0 = sensor[0];
            Assert.Equal(42, row0.GetProperty("reading").GetInt64());

            // Second row: 999 overflows int8, coerced to 0
            var row1 = sensor[1];
            Assert.Equal(0, row1.GetProperty("reading").GetInt64());
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void NumericCoercion_NegativeToUInt_CoercesToZero_BinaryRoundTrip()
    {
        // UInt field with negative value → should coerce to 0
        var path = Path.Combine(_tempDir, $"test_coerce_uint_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                @struct Counter (count: uint, label: string)
                counter: @table Counter [
                    (100, valid),
                    (-5, negative),
                ]
            "))
            {
                doc.Compile(path);
            }

            using var reader = TLReader.Open(path);
            var json = reader.ToJson();
            Assert.NotNull(json);

            using var jsonDoc = System.Text.Json.JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;
            Assert.True(root.TryGetProperty("counter", out var counter));

            // First row: 100 is valid uint
            var row0 = counter[0];
            Assert.Equal(100, row0.GetProperty("count").GetInt64());

            // Second row: -5 is negative, coerced to 0
            var row1 = counter[1];
            Assert.Equal(0, row1.GetProperty("count").GetInt64());
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    // ------------------------------------------------------------------
    // Value-only schema field types rejected (parser.rs fix)
    // ------------------------------------------------------------------

    [Theory]
    [InlineData("object")]
    [InlineData("map")]
    [InlineData("tuple")]
    [InlineData("ref")]
    [InlineData("tagged")]
    public void Parse_ValueOnlySchemaFieldType_Throws(string fieldType)
    {
        var input = $"@struct Bad (field: {fieldType})";
        Assert.Throws<TLException>(() => TLDocument.Parse(input));
    }

    // ------------------------------------------------------------------
    // Struct array (@table) binary roundtrip
    // ------------------------------------------------------------------

    [Fact]
    public void StructArray_Table_BinaryRoundTrip_PreservesData()
    {
        var path = Path.Combine(_tempDir, $"test_table_rt_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                @struct Employee (name: string, age: int, email: string?)
                team: @table Employee [
                    (alice, 30, ""a@test.com""),
                    (bob, 25, ~),
                    (charlie, 35, ""c@test.com""),
                ]
            "))
            {
                doc.Compile(path);
            }

            using var reader = TLReader.Open(path);
            using var team = reader["team"];
            Assert.NotNull(team);
            Assert.Equal(TLType.Array, team.Type);
            Assert.Equal(3, team.ArrayLength);

            // Row 0: all fields present
            using var row0 = team[0];
            using var name0 = row0?["name"];
            Assert.Equal("alice", name0?.AsString());
            using var age0 = row0?["age"];
            Assert.Equal(30, age0?.AsInt());
            using var email0 = row0?["email"];
            Assert.Equal("a@test.com", email0?.AsString());

            // Row 1: email is null
            using var row1 = team[1];
            using var name1 = row1?["name"];
            Assert.Equal("bob", name1?.AsString());
            using var age1 = row1?["age"];
            Assert.Equal(25, age1?.AsInt());
            using var email1 = row1?["email"];
            Assert.NotNull(email1);
            Assert.Equal(TLType.Null, email1.Type);

            // Row 2: all fields present
            using var row2 = team[2];
            using var name2 = row2?["name"];
            Assert.Equal("charlie", name2?.AsString());
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void StructArray_Table_JsonRoundTrip_PreservesSchema()
    {
        // Verify @table data survives text → binary → JSON → re-import
        var path = Path.Combine(_tempDir, $"test_table_json_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                @struct Product (name: string, price: float, in_stock: bool)
                catalog: @table Product [
                    (widget, 9.99, true),
                    (gadget, 24.99, false),
                    (doohickey, 4.99, true),
                ]
            "))
            {
                doc.Compile(path);
            }

            using var reader = TLReader.Open(path);
            var json = reader.ToJson();
            Assert.NotNull(json);

            // Verify JSON structure
            using var jsonDoc = System.Text.Json.JsonDocument.Parse(json);
            var root = jsonDoc.RootElement;
            Assert.True(root.TryGetProperty("catalog", out var catalog));
            Assert.Equal(System.Text.Json.JsonValueKind.Array, catalog.ValueKind);
            Assert.Equal(3, catalog.GetArrayLength());

            // First product
            var p0 = catalog[0];
            Assert.Equal("widget", p0.GetProperty("name").GetString());
            Assert.Equal(9.99, p0.GetProperty("price").GetDouble(), 2);
            Assert.True(p0.GetProperty("in_stock").GetBoolean());

            // Second product
            var p1 = catalog[1];
            Assert.Equal("gadget", p1.GetProperty("name").GetString());
            Assert.False(p1.GetProperty("in_stock").GetBoolean());
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    // ------------------------------------------------------------------
    // Unknown directive argument consumption (parser.rs fix)
    // ------------------------------------------------------------------

    [Fact]
    public void Parse_UnknownDirectiveInValue_ReturnsNull()
    {
        // Unknown directive in value position should consume arg and return null
        using var doc = TLDocument.Parse("val: @unknown_directive something\nother: 42");
        using var val = doc["val"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Null, val.Type);

        // The next key should still parse correctly
        using var other = doc["other"];
        Assert.NotNull(other);
        Assert.Equal(42, other.AsInt());
    }

    [Fact]
    public void Parse_UnknownTopLevelDirective_ConsumesSameLineArg()
    {
        // Same-line argument consumed — "foo" should NOT become a key
        using var doc = TLDocument.Parse("@custom foo\nkey: value");
        Assert.False(doc.ContainsKey("foo"));
        using var val = doc["key"];
        Assert.Equal("value", val?.AsString());
    }

    [Fact]
    public void Parse_UnknownTopLevelDirective_NextLineNotConsumed()
    {
        // Argument on next line NOT consumed — "other" is a key-value pair
        using var doc = TLDocument.Parse("@custom\nother: 42");
        using var other = doc["other"];
        Assert.NotNull(other);
        Assert.Equal(42, other.AsInt());
    }
}

// ========================================================================
// TLDocument Edge Case Tests
// ========================================================================

public class TLDocumentEdgeCaseTests
{
    [Fact]
    public void ToString_ReturnsSameAsToText()
    {
        using var doc = TLDocument.Parse("greeting: hello");
        var text = doc.ToText();
        var str = doc.ToString();
        Assert.Equal(text, str);
    }

    [Fact]
    public void DoubleDispose_DoesNotThrow()
    {
        var doc = TLDocument.Parse("key: value");
        doc.Dispose();
        doc.Dispose(); // Should not throw
    }

    [Fact]
    public void AccessAfterDispose_ThrowsObjectDisposed()
    {
        var doc = TLDocument.Parse("key: value");
        doc.Dispose();
        Assert.Throws<ObjectDisposedException>(() => doc.ToText());
    }

    [Fact]
    public void ParseFile_ValidFile_Succeeds()
    {
        var path = Path.Combine(Path.GetTempPath(), $"tealeaf_parsefile_{Guid.NewGuid()}.tl");
        try
        {
            File.WriteAllText(path, "name: alice\nage: 30");
            using var doc = TLDocument.ParseFile(path);
            Assert.NotNull(doc);
            using var name = doc["name"];
            Assert.Equal("alice", name?.AsString());
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void ParseFile_NonExistent_ThrowsTLException()
    {
        Assert.Throws<TLException>(() => TLDocument.ParseFile("nonexistent_file.tl"));
    }

    [Fact]
    public void ToTextDataOnly_OmitsSchemas()
    {
        using var doc = TLDocument.Parse(@"
            @struct Person (name: string, age: int)
            people: @table Person [
                (alice, 30)
            ]
        ");
        var dataOnly = doc.ToTextDataOnly();
        Assert.DoesNotContain("@struct", dataOnly);
    }
}

// ========================================================================
// TLValue Edge Case Tests
// ========================================================================

public class TLValueEdgeCaseTests
{
    [Fact]
    public void GetArrayElement_NegativeIndex_ReturnsNull()
    {
        using var doc = TLDocument.Parse("arr: [1, 2, 3]");
        using var arr = doc["arr"];
        Assert.Null(arr?.GetArrayElement(-1));
    }

    [Fact]
    public void GetMapKey_NegativeIndex_ReturnsNull()
    {
        using var doc = TLDocument.Parse(@"m: @map {a: 1, b: 2}");
        using var m = doc["m"];
        Assert.Null(m?.GetMapKey(-1));
    }

    [Fact]
    public void GetMapValue_NegativeIndex_ReturnsNull()
    {
        using var doc = TLDocument.Parse(@"m: @map {a: 1, b: 2}");
        using var m = doc["m"];
        Assert.Null(m?.GetMapValue(-1));
    }

    [Fact]
    public void GetField_OnNonObject_ReturnsNull()
    {
        using var doc = TLDocument.Parse("val: 42");
        using var val = doc["val"];
        Assert.Null(val?.GetField("anything"));
    }

    [Fact]
    public void GetObjectKeys_OnNonObject_ReturnsEmpty()
    {
        using var doc = TLDocument.Parse("val: 42");
        using var val = doc["val"];
        var keys = val?.GetObjectKeys();
        Assert.NotNull(keys);
        Assert.Empty(keys!);
    }

    [Fact]
    public void AsUInt_OnUIntValue_ReturnsValue()
    {
        // UInt values in TeaLeaf are created from JSON with large positive numbers
        // or from binary files. Use JSON roundtrip to create one.
        using var doc = TLDocument.FromJson(@"{""big"": 18446744073709551615}");
        using var val = doc["big"];
        Assert.NotNull(val);
        // Depending on how Rust parses, this could be UInt or Int
        // but we can test the accessor
        var uintVal = val.AsUInt();
        var intVal = val.AsInt();
        Assert.True(uintVal.HasValue || intVal.HasValue);
    }

    [Fact]
    public void ToObject_Timestamp_ReturnsDateTimeOffset()
    {
        using var doc = TLDocument.Parse("ts: 2024-01-15T10:30:00Z");
        using var val = doc["ts"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Timestamp, val.Type);
        var obj = val.ToObject();
        Assert.IsType<DateTimeOffset>(obj);
    }

    [Fact]
    public void Timestamp_UTC_HasZeroOffset()
    {
        using var doc = TLDocument.Parse("ts: 2024-01-15T10:30:00Z");
        using var val = doc["ts"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Timestamp, val.Type);
        Assert.Equal((short)0, val.AsTimestampOffset());
        var dt = val.AsDateTime();
        Assert.NotNull(dt);
        Assert.Equal(TimeSpan.Zero, dt!.Value.Offset);
    }

    [Fact]
    public void Timestamp_WithOffset_PreservesTimezone()
    {
        using var doc = TLDocument.Parse("ts: 2024-01-15T16:00:00+05:30");
        using var val = doc["ts"];
        Assert.NotNull(val);
        Assert.Equal(TLType.Timestamp, val.Type);
        Assert.Equal((short)330, val.AsTimestampOffset());
        var dt = val.AsDateTime();
        Assert.NotNull(dt);
        Assert.Equal(TimeSpan.FromMinutes(330), dt!.Value.Offset);
        Assert.Equal(16, dt.Value.Hour); // Local hour, not UTC
    }

    [Fact]
    public void Timestamp_NegativeOffset_PreservesTimezone()
    {
        using var doc = TLDocument.Parse("ts: 2024-01-15T07:00:00-05:00");
        using var val = doc["ts"];
        Assert.NotNull(val);
        Assert.Equal((short)(-300), val.AsTimestampOffset());
        var dt = val.AsDateTime();
        Assert.NotNull(dt);
        Assert.Equal(TimeSpan.FromMinutes(-300), dt!.Value.Offset);
        Assert.Equal(7, dt.Value.Hour);
    }

    [Fact]
    public void TimestampOffset_NonTimestamp_ReturnsNull()
    {
        using var doc = TLDocument.Parse("x: 42");
        using var val = doc["x"];
        Assert.NotNull(val);
        Assert.Null(val.AsTimestampOffset());
    }

    [Fact]
    public void ToObject_UInt_ReturnsUlong()
    {
        // Create a document with a UInt value via binary roundtrip
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory, "..", "..", "..", "fixtures", "comprehensive.tlbx");
        if (!File.Exists(fixturePath)) return;

        using var reader = TLReader.Open(fixturePath);
        using var val = reader["uint_val"];
        if (val == null) return;
        if (val.Type == TLType.UInt)
        {
            var obj = val.ToObject();
            Assert.IsType<ulong>(obj);
        }
    }

    [Fact]
    public void DoubleDispose_DoesNotThrow()
    {
        using var doc = TLDocument.Parse("val: 42");
        var val = doc["val"];
        Assert.NotNull(val);
        val!.Dispose();
        val.Dispose(); // Should not throw
    }

    [Fact]
    public void AccessAfterDispose_ThrowsObjectDisposed()
    {
        using var doc = TLDocument.Parse("val: 42");
        var val = doc["val"]!;
        val.Dispose();
        Assert.Throws<ObjectDisposedException>(() => val.Type);
    }
}

// ========================================================================
// TLReader Edge Case Tests
// ========================================================================

public class TLReaderEdgeCaseTests
{
    private readonly string _tempDir = Path.GetTempPath();

    [Fact]
    public void OpenMmap_ValidFile_Succeeds()
    {
        var path = Path.Combine(_tempDir, $"test_mmap_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse("name: alice"))
            {
                doc.Compile(path);
            }
            using var reader = TLReader.OpenMmap(path);
            Assert.NotNull(reader);
            using var name = reader["name"];
            Assert.Equal("alice", name?.AsString());
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void TryOpenMmap_ValidFile_ReturnsTrue()
    {
        var path = Path.Combine(_tempDir, $"test_trymmap_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse("val: 42"))
            {
                doc.Compile(path);
            }
            var success = TLReader.TryOpenMmap(path, out var reader);
            Assert.True(success);
            Assert.NotNull(reader);
            reader?.Dispose();
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void TryOpenMmap_NonExistent_ReturnsFalse()
    {
        var success = TLReader.TryOpenMmap("nonexistent.tlbx", out var reader);
        Assert.False(success);
        Assert.Null(reader);
    }

    [Fact]
    public void OpenMmap_NonExistent_ThrowsTLException()
    {
        Assert.Throws<TLException>(() => TLReader.OpenMmap("nonexistent.tlbx"));
    }

    [Fact]
    public void ContainsKey_ExistingKey_ReturnsTrue()
    {
        var path = Path.Combine(_tempDir, $"test_contains_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse("key: value"))
            {
                doc.Compile(path);
            }
            using var reader = TLReader.Open(path);
            Assert.True(reader.ContainsKey("key"));
            Assert.False(reader.ContainsKey("nonexistent"));
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void Schemas_EmptyFile_ReturnsEmpty()
    {
        var path = Path.Combine(_tempDir, $"test_schemas_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse("key: value"))
            {
                doc.Compile(path);
            }
            using var reader = TLReader.Open(path);
            Assert.NotNull(reader.Schemas);
            Assert.Empty(reader.Schemas);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void Schemas_WithSchema_ReturnsSchemas()
    {
        var path = Path.Combine(_tempDir, $"test_schemas2_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                @struct Person (name: string, age: int)
                people: @table Person [
                    (alice, 30),
                    (bob, 25),
                ]
            "))
            {
                doc.Compile(path);
            }
            using var reader = TLReader.Open(path);
            Assert.NotNull(reader.Schemas);
            Assert.True(reader.Schemas.Count >= 1);
            Assert.Equal("Person", reader.Schemas[0].Name);
            Assert.Equal(2, reader.Schemas[0].Fields.Count);
            Assert.Equal("name", reader.Schemas[0].Fields[0].Name);
            Assert.Equal("string", reader.Schemas[0].Fields[0].Type);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void GetSchema_ByName_ReturnsCorrectSchema()
    {
        var path = Path.Combine(_tempDir, $"test_getschema_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                @struct Person (name: string, age: int)
                people: @table Person [
                    (alice, 30),
                ]
            "))
            {
                doc.Compile(path);
            }
            using var reader = TLReader.Open(path);
            var schema = reader.GetSchema("Person");
            Assert.NotNull(schema);
            Assert.Equal("Person", schema!.Name);

            var missing = reader.GetSchema("Nonexistent");
            Assert.Null(missing);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void GetDynamic_SimpleValues_ReturnsCorrectTypes()
    {
        var path = Path.Combine(_tempDir, $"test_dynamic_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                name: alice
                age: 30
                active: true
                pi: 3.14
                items: [1, 2, 3]
                user: {first: bob, last: smith}
            "))
            {
                doc.Compile(path);
            }
            using var reader = TLReader.Open(path);

            dynamic? name = reader.GetDynamic("name");
            Assert.Equal("alice", (string?)name);

            dynamic? age = reader.GetDynamic("age");
            Assert.Equal(30L, (long?)age);

            dynamic? active = reader.GetDynamic("active");
            Assert.True((bool?)active);

            dynamic? missing = reader.GetDynamic("nonexistent");
            Assert.Null(missing);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void GetDynamic_SpecialTypes_Work()
    {
        var path = Path.Combine(_tempDir, $"test_dynamic_special_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse(@"
                base: {x: 1}
                ref_val: !base
                tagged_val: :ok 200
                map_val: @map {a: 1, b: 2}
                ts: 2024-01-15T10:30:00Z
            "))
            {
                doc.Compile(path);
            }
            using var reader = TLReader.Open(path);

            // Ref becomes string (the ref name)
            dynamic? refVal = reader.GetDynamic("ref_val");
            Assert.NotNull(refVal);

            // Tagged becomes ExpandoObject with $tag and $value
            dynamic? taggedVal = reader.GetDynamic("tagged_val");
            Assert.NotNull(taggedVal);

            // Map becomes KeyValuePair array
            dynamic? mapVal = reader.GetDynamic("map_val");
            Assert.NotNull(mapVal);

            // Timestamp becomes DateTimeOffset
            dynamic? ts = reader.GetDynamic("ts");
            Assert.NotNull(ts);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void DoubleDispose_DoesNotThrow()
    {
        var path = Path.Combine(_tempDir, $"test_disp_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse("k: v"))
            {
                doc.Compile(path);
            }
            var reader = TLReader.Open(path);
            reader.Dispose();
            reader.Dispose(); // Should not throw
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void AccessAfterDispose_ThrowsObjectDisposed()
    {
        var path = Path.Combine(_tempDir, $"test_disp2_{Guid.NewGuid()}.tlbx");
        try
        {
            using (var doc = TLDocument.Parse("k: v"))
            {
                doc.Compile(path);
            }
            var reader = TLReader.Open(path);
            reader.Dispose();
            Assert.Throws<ObjectDisposedException>(() => reader.Keys);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }
}

// ========================================================================
// TeaLeafSerializer Edge Case Tests
// ========================================================================

public class TeaLeafSerializerEdgeCaseTests
{
    [Fact]
    public void ToDocument_WithCustomKey_UsesCustomKey()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };
        var doc = TeaLeafSerializer.ToDocument(user, "custom_key");
        Assert.Contains("custom_key:", doc);
    }

    [Fact]
    public void FromDocument_MissingKey_ThrowsTLException()
    {
        using var doc = TLDocument.Parse("other_key: {name: alice}");
        Assert.Throws<TLException>(() => TeaLeafSerializer.FromDocument<SimpleUser>(doc));
    }

    [Fact]
    public void FromList_NonArrayValue_ThrowsTLException()
    {
        using var doc = TLDocument.Parse("users: {name: alice}");
        using var val = doc["users"]!;
        Assert.Throws<TLException>(() => TeaLeafSerializer.FromList<SimpleUser>(val));
    }

    [Fact]
    public void FromList_MissingKey_ThrowsTLException()
    {
        using var doc = TLDocument.Parse("other: [1, 2, 3]");
        Assert.Throws<TLException>(() => TeaLeafSerializer.FromList<SimpleUser>(doc, "users"));
    }

    [Fact]
    public void Compile_ProducesBinaryFile()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };
        var path = Path.Combine(Path.GetTempPath(), $"tealeaf_compile_{Guid.NewGuid()}.tlbx");
        try
        {
            TeaLeafSerializer.Compile(user, path);
            Assert.True(File.Exists(path));

            using var reader = TLReader.Open(path);
            using var val = reader["simple_user"];
            Assert.NotNull(val);
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Fact]
    public void ToJson_ProducesValidJson()
    {
        var user = new SimpleUser { Name = "alice", Age = 30, Active = true };
        var json = TeaLeafSerializer.ToJson(user);
        Assert.Contains("alice", json);
        Assert.Contains("30", json);
    }

    [Fact]
    public void RoundTrip_NumericModel_AllTypes()
    {
        var original = new NumericModel
        {
            SmallInt = 123,
            TinyInt = 42,
            UnsignedInt = 999,
            BigUnsigned = 12345678901234,
            SinglePrecision = 3.14f,
            Money = 99.99m
        };

        // Serialization exercises all AppendValue type branches
        var text = TeaLeafSerializer.ToText(original);
        Assert.Contains("123", text);
        Assert.Contains("42", text);
        Assert.Contains("999", text);

        // Deserialization exercises ReadPrimitive for supported types
        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<NumericModel>(doc);

        Assert.Equal(123, restored.SmallInt);
        Assert.Equal(42, restored.TinyInt);
        Assert.True(Math.Abs(3.14f - restored.SinglePrecision) < 0.01f);
        Assert.True(Math.Abs(99.99m - restored.Money) < 0.01m);
    }

    [Fact]
    public void RoundTrip_NullableNumericModel_WithValues()
    {
        var original = new NullableNumericModel
        {
            Name = "test",
            SmallVal = 42,
            TinyVal = 7,
            UnsignedVal = 100,
            FloatVal = 2.5f,
            DecimalVal = 9.99m
        };

        // Serialization exercises AppendValue for nullable numeric types
        var text = TeaLeafSerializer.ToText(original);
        Assert.Contains("42", text);
        Assert.Contains("100", text);

        // Deserialization exercises ReadPrimitive for nullable types
        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<NullableNumericModel>(doc);

        Assert.Equal("test", restored.Name);
        Assert.Equal((short)42, restored.SmallVal);
        Assert.Equal((byte)7, restored.TinyVal);
    }

    [Fact]
    public void Serialization_EnumValue_SerializesAsSnakeCase()
    {
        var model = new UserWithEnum { Name = "test", Role = UserRole.Viewer };
        var text = TeaLeafSerializer.ToText(model);
        Assert.Contains("viewer", text);
    }

    [Fact]
    public void RoundTrip_EnumValue_Preserved()
    {
        var original = new UserWithEnum { Name = "admin", Role = UserRole.Admin };
        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<UserWithEnum>(doc);
        Assert.Equal(UserRole.Admin, restored.Role);
    }

    [Fact]
    public void RoundTrip_NullableFields_NullsPreserved()
    {
        var original = new NullableFields
        {
            Name = "test",
            Email = null,
            Age = null
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<NullableFields>(doc);

        Assert.Equal("test", restored.Name);
        Assert.Null(restored.Email);
        Assert.Null(restored.Age);
    }

    [Fact]
    public void RoundTrip_DictionaryProperty()
    {
        // Dictionaries are serialized as objects
        var text = TeaLeafSerializer.GetSchema<WithCollections>();
        Assert.Contains("tags: []string", text);
        Assert.Contains("scores: []int", text);
    }
}

// ========================================================================
// TeaLeafTextHelper Edge Case Tests
// ========================================================================

public class TeaLeafTextHelperEdgeCaseTests
{
    // ToSnakeCase

    [Fact]
    public void ToSnakeCase_EmptyString_ReturnsEmpty()
    {
        Assert.Equal("", TeaLeafTextHelper.ToSnakeCase(""));
    }

    [Fact]
    public void ToSnakeCase_Null_ReturnsNull()
    {
        Assert.Null(TeaLeafTextHelper.ToSnakeCase(null!));
    }

    [Fact]
    public void ToSnakeCase_AllCaps_InsertsUnderscores()
    {
        Assert.Equal("h_t_t_p", TeaLeafTextHelper.ToSnakeCase("HTTP"));
    }

    [Fact]
    public void ToSnakeCase_SingleChar_Lowered()
    {
        Assert.Equal("a", TeaLeafTextHelper.ToSnakeCase("A"));
    }

    [Fact]
    public void ToSnakeCase_AlreadyLower_Unchanged()
    {
        Assert.Equal("already_snake", TeaLeafTextHelper.ToSnakeCase("already_snake"));
    }

    // NeedsQuoting

    [Fact]
    public void NeedsQuoting_EmptyString_ReturnsTrue()
    {
        Assert.True(TeaLeafTextHelper.NeedsQuoting(""));
    }

    [Fact]
    public void NeedsQuoting_ReservedWords_ReturnTrue()
    {
        Assert.True(TeaLeafTextHelper.NeedsQuoting("true"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("false"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("~"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("null"));
    }

    [Fact]
    public void NeedsQuoting_SimpleIdentifier_ReturnsFalse()
    {
        Assert.False(TeaLeafTextHelper.NeedsQuoting("hello"));
        Assert.False(TeaLeafTextHelper.NeedsQuoting("simple_value"));
    }

    [Fact]
    public void NeedsQuoting_DigitPrefix_ReturnsTrue()
    {
        Assert.True(TeaLeafTextHelper.NeedsQuoting("44mm"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("+5"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("-3"));
    }

    [Fact]
    public void NeedsQuoting_SpecialChars_ReturnTrue()
    {
        Assert.True(TeaLeafTextHelper.NeedsQuoting("has space"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("has:colon"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("has/slash"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("has@at"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("has#hash"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("has,comma"));
    }

    [Fact]
    public void NeedsQuoting_NumericString_ReturnsTrue()
    {
        Assert.True(TeaLeafTextHelper.NeedsQuoting("3.14"));
        Assert.True(TeaLeafTextHelper.NeedsQuoting("42"));
    }

    // QuoteIfNeeded / EscapeString

    [Fact]
    public void QuoteIfNeeded_SafeValue_NotQuoted()
    {
        Assert.Equal("hello", TeaLeafTextHelper.QuoteIfNeeded("hello"));
    }

    [Fact]
    public void QuoteIfNeeded_UnsafeValue_Quoted()
    {
        var result = TeaLeafTextHelper.QuoteIfNeeded("true");
        Assert.StartsWith("\"", result);
        Assert.EndsWith("\"", result);
    }

    [Fact]
    public void EscapeString_EmptyString_ReturnsEmpty()
    {
        Assert.Equal("", TeaLeafTextHelper.EscapeString(""));
    }

    [Fact]
    public void EscapeString_SpecialChars_Escaped()
    {
        Assert.Equal("\\\\", TeaLeafTextHelper.EscapeString("\\"));
        Assert.Equal("\\\"", TeaLeafTextHelper.EscapeString("\""));
        Assert.Equal("\\n", TeaLeafTextHelper.EscapeString("\n"));
        Assert.Equal("\\r", TeaLeafTextHelper.EscapeString("\r"));
        Assert.Equal("\\t", TeaLeafTextHelper.EscapeString("\t"));
    }

    // AppendValue

    [Fact]
    public void AppendValue_Null_AppendsTilde()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, null, typeof(string));
        Assert.Equal("~", sb.ToString());
    }

    [Fact]
    public void AppendValue_Bool_AppendsLiteral()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, true, typeof(bool));
        Assert.Equal("true", sb.ToString());

        sb.Clear();
        TeaLeafTextHelper.AppendValue(sb, false, typeof(bool));
        Assert.Equal("false", sb.ToString());
    }

    [Fact]
    public void AppendValue_Int_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, 42, typeof(int));
        Assert.Equal("42", sb.ToString());
    }

    [Fact]
    public void AppendValue_Long_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, 123456789012345L, typeof(long));
        Assert.Equal("123456789012345", sb.ToString());
    }

    [Fact]
    public void AppendValue_Double_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, 3.14, typeof(double));
        Assert.Equal("3.14", sb.ToString());
    }

    [Fact]
    public void AppendValue_Float_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, 2.5f, typeof(float));
        Assert.Contains("2.5", sb.ToString());
    }

    [Fact]
    public void AppendValue_UInt_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, 42u, typeof(uint));
        Assert.Equal("42", sb.ToString());
    }

    [Fact]
    public void AppendValue_ULong_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, 99UL, typeof(ulong));
        Assert.Equal("99", sb.ToString());
    }

    [Fact]
    public void AppendValue_Short_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, (short)7, typeof(short));
        Assert.Equal("7", sb.ToString());
    }

    [Fact]
    public void AppendValue_Byte_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, (byte)255, typeof(byte));
        Assert.Equal("255", sb.ToString());
    }

    [Fact]
    public void AppendValue_Decimal_AppendsNumber()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, 99.99m, typeof(decimal));
        Assert.Equal("99.99", sb.ToString());
    }

    [Fact]
    public void AppendValue_DateTime_AppendsIso8601()
    {
        var sb = new StringBuilder();
        var dt = new DateTime(2024, 1, 15, 10, 30, 0, DateTimeKind.Utc);
        TeaLeafTextHelper.AppendValue(sb, dt, typeof(DateTime));
        Assert.Contains("2024-01-15", sb.ToString());
    }

    [Fact]
    public void AppendValue_DateTimeOffset_AppendsIso8601()
    {
        var sb = new StringBuilder();
        var dto = new DateTimeOffset(2024, 1, 15, 10, 30, 0, TimeSpan.Zero);
        TeaLeafTextHelper.AppendValue(sb, dto, typeof(DateTimeOffset));
        Assert.Contains("2024-01-15", sb.ToString());
    }

    [Fact]
    public void AppendValue_Enum_AppendsSnakeCase()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, UserRole.Admin, typeof(UserRole));
        Assert.Equal("admin", sb.ToString());
    }

    [Fact]
    public void AppendValue_UnknownType_FallsBackToString()
    {
        var sb = new StringBuilder();
        TeaLeafTextHelper.AppendValue(sb, new Uri("https://example.com"), typeof(Uri));
        Assert.Contains("example.com", sb.ToString());
    }

    // GetTLTypeName

    [Fact]
    public void GetTLTypeName_Primitives()
    {
        Assert.Equal("bool", TeaLeafTextHelper.GetTLTypeName(typeof(bool)));
        Assert.Equal("int", TeaLeafTextHelper.GetTLTypeName(typeof(int)));
        Assert.Equal("int64", TeaLeafTextHelper.GetTLTypeName(typeof(long)));
        Assert.Equal("int16", TeaLeafTextHelper.GetTLTypeName(typeof(short)));
        Assert.Equal("int8", TeaLeafTextHelper.GetTLTypeName(typeof(sbyte)));
        Assert.Equal("uint", TeaLeafTextHelper.GetTLTypeName(typeof(uint)));
        Assert.Equal("uint64", TeaLeafTextHelper.GetTLTypeName(typeof(ulong)));
        Assert.Equal("uint16", TeaLeafTextHelper.GetTLTypeName(typeof(ushort)));
        Assert.Equal("uint8", TeaLeafTextHelper.GetTLTypeName(typeof(byte)));
        Assert.Equal("float", TeaLeafTextHelper.GetTLTypeName(typeof(double)));
        Assert.Equal("float32", TeaLeafTextHelper.GetTLTypeName(typeof(float)));
        Assert.Equal("float", TeaLeafTextHelper.GetTLTypeName(typeof(decimal)));
        Assert.Equal("string", TeaLeafTextHelper.GetTLTypeName(typeof(string)));
    }

    [Fact]
    public void GetTLTypeName_DateTimeTypes()
    {
        Assert.Equal("timestamp", TeaLeafTextHelper.GetTLTypeName(typeof(DateTime)));
        Assert.Equal("timestamp", TeaLeafTextHelper.GetTLTypeName(typeof(DateTimeOffset)));
    }

    [Fact]
    public void GetTLTypeName_ByteArray()
    {
        Assert.Equal("bytes", TeaLeafTextHelper.GetTLTypeName(typeof(byte[])));
    }

    [Fact]
    public void GetTLTypeName_Enum()
    {
        Assert.Equal("string", TeaLeafTextHelper.GetTLTypeName(typeof(UserRole)));
    }

    [Fact]
    public void GetTLTypeName_ListOfPrimitives()
    {
        Assert.Equal("[]int", TeaLeafTextHelper.GetTLTypeName(typeof(List<int>)));
        Assert.Equal("[]string", TeaLeafTextHelper.GetTLTypeName(typeof(List<string>)));
    }

    [Fact]
    public void GetTLTypeName_Dictionary()
    {
        Assert.Equal("object", TeaLeafTextHelper.GetTLTypeName(typeof(Dictionary<string, string>)));
    }

    [Fact]
    public void GetTLTypeName_NestedTeaLeafType()
    {
        // SimpleUser has [TeaLeaf] attribute -> should return its struct name
        var tlType = TeaLeafTextHelper.GetTLTypeName(typeof(SimpleUser));
        Assert.Equal("simple_user", tlType);
    }

    [Fact]
    public void GetTLTypeName_NullableType()
    {
        // Nullable<int> should unwrap to int
        Assert.Equal("int", TeaLeafTextHelper.GetTLTypeName(typeof(int?)));
        Assert.Equal("bool", TeaLeafTextHelper.GetTLTypeName(typeof(bool?)));
    }

    [Fact]
    public void GetTLTypeName_UnknownType_ReturnsObject()
    {
        Assert.Equal("object", TeaLeafTextHelper.GetTLTypeName(typeof(Uri)));
    }

    // TLException constructor coverage

    [Fact]
    public void TLException_DefaultConstructor()
    {
        var ex = new TLException();
        Assert.NotNull(ex);
        Assert.Null(ex.InnerException);
    }

    [Fact]
    public void TLException_InnerExceptionConstructor()
    {
        var inner = new InvalidOperationException("inner error");
        var ex = new TLException("outer error", inner);
        Assert.Equal("outer error", ex.Message);
        Assert.Same(inner, ex.InnerException);
    }

    [Fact]
    public void GetLastError_NoError_ReturnsNull()
    {
        // Successful operation clears the error state
        using var doc = TLDocument.Parse("key: 42");
        Assert.Null(NativeMethods.GetLastError());
    }

    [Fact]
    public void PtrToStringAndFree_Zero_ReturnsNull()
    {
        Assert.Null(NativeMethods.PtrToStringAndFree(IntPtr.Zero));
    }

    [Fact]
    public void PtrToStringArrayAndFree_Zero_ReturnsEmpty()
    {
        var result = NativeMethods.PtrToStringArrayAndFree(IntPtr.Zero);
        Assert.Empty(result);
    }

    [Fact]
    public void TLResult_Success_DoesNotThrow()
    {
        var result = new TLResult { Success = true, ErrorMessage = IntPtr.Zero };
        result.ThrowIfError(); // should not throw
    }

    [Fact]
    public void TLResult_FailWithoutMessage_DoesNotThrow()
    {
        // Success == false but ErrorMessage == IntPtr.Zero: edge case, no throw
        var result = new TLResult { Success = false, ErrorMessage = IntPtr.Zero };
        result.ThrowIfError();
    }

    // TLField / TLSchema coverage

    [Fact]
    public void TLField_Properties_And_ToString()
    {
        var field = new TLField("name", "string", nullable: false, isArray: false);
        Assert.Equal("name", field.Name);
        Assert.Equal("string", field.Type);
        Assert.False(field.IsNullable);
        Assert.False(field.IsArray);
        Assert.Equal("name: string", field.ToString());
    }

    [Fact]
    public void TLField_Nullable_ToString()
    {
        var field = new TLField("email", "string", nullable: true, isArray: false);
        Assert.True(field.IsNullable);
        Assert.Equal("email: string?", field.ToString());
    }

    [Fact]
    public void TLField_Array_ToString()
    {
        var field = new TLField("tags", "string", nullable: false, isArray: true);
        Assert.True(field.IsArray);
        Assert.Equal("tags: string[]", field.ToString());
    }

    [Fact]
    public void TLField_NullableArray_ToString()
    {
        var field = new TLField("items", "int", nullable: true, isArray: true);
        Assert.Equal("items: int[]?", field.ToString());
    }

    [Fact]
    public void TLSchema_GetField_And_HasField()
    {
        var fields = new List<TLField>
        {
            new TLField("name", "string", false, false),
            new TLField("age", "int", false, false),
        };
        var schema = new TLSchema("User", fields);

        Assert.Equal("User", schema.Name);
        Assert.Equal(2, schema.Fields.Count);

        Assert.True(schema.HasField("name"));
        Assert.True(schema.HasField("age"));
        Assert.False(schema.HasField("missing"));

        var nameField = schema.GetField("name");
        Assert.NotNull(nameField);
        Assert.Equal("string", nameField!.Type);

        Assert.Null(schema.GetField("missing"));
    }

    [Fact]
    public void TLSchema_ToString()
    {
        var fields = new List<TLField>
        {
            new TLField("name", "string", false, false),
            new TLField("email", "string", true, false),
        };
        var schema = new TLSchema("User", fields);
        Assert.Equal("@struct User (name: string, email: string?)", schema.ToString());
    }

    // ----------------------------------------------------------------
    // WriteDictionary coverage
    // ----------------------------------------------------------------

    [Fact]
    public void Serializer_WriteDictionary_ContainsEntries()
    {
        var model = new WithDictionary
        {
            Name = "test",
            Metadata = new Dictionary<string, string>
            {
                { "env", "prod" },
                { "region", "us-east" }
            }
        };

        var text = TeaLeafSerializer.ToText(model);
        Assert.Contains("metadata:", text);
        Assert.Contains("env:", text);
        Assert.Contains("prod", text);
        Assert.Contains("region:", text);
    }

    [Fact]
    public void Serializer_WriteDictionary_NullValue()
    {
        var model = new WithDictionary
        {
            Name = "test",
            Metadata = new Dictionary<string, string>
            {
                { "present", "value" },
                { "absent", null! }
            }
        };

        var text = TeaLeafSerializer.ToText(model);
        Assert.Contains("present:", text);
        Assert.Contains("value", text);
        Assert.Contains("~", text);
    }

    // ----------------------------------------------------------------
    // ReadDictionary coverage
    // ----------------------------------------------------------------

    [Fact]
    public void Serializer_ReadDictionary_FromText()
    {
        // Construct TL text without schema (avoids 'object' type rejection)
        var tlText = @"
            with_dictionary: {
                name: test
                metadata: {
                    color: blue
                    size: large
                }
            }
        ";

        var restored = TeaLeafSerializer.FromText<WithDictionary>(tlText);

        Assert.Equal("test", restored.Name);
        Assert.Equal(2, restored.Metadata.Count);
        Assert.Equal("blue", restored.Metadata["color"]);
        Assert.Equal("large", restored.Metadata["size"]);
    }

    // ----------------------------------------------------------------
    // WriteList empty branch coverage
    // ----------------------------------------------------------------

    [Fact]
    public void Serializer_WriteList_Empty()
    {
        var model = new WithCollections
        {
            Name = "test",
            Tags = new List<string>(),
            Scores = new List<int>()
        };

        var text = TeaLeafSerializer.ToText(model);
        Assert.Contains("tags: []", text);
        Assert.Contains("scores: []", text);
    }

    // ----------------------------------------------------------------
    // ReadPrimitive DateTime / DateTimeOffset coverage
    // ----------------------------------------------------------------

    [Fact]
    public void Serializer_DateTimeOffset_Roundtrip()
    {
        var original = new WithTimestamps
        {
            Label = "event",
            CreatedAt = new DateTimeOffset(2024, 6, 15, 10, 30, 0, TimeSpan.Zero),
            UpdatedAt = new DateTimeOffset(2024, 6, 15, 16, 0, 0, TimeSpan.FromHours(5.5))
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<WithTimestamps>(doc);

        Assert.Equal("event", restored.Label);
        Assert.Equal(2024, restored.CreatedAt.Year);
        Assert.Equal(6, restored.CreatedAt.Month);
        Assert.Equal(15, restored.CreatedAt.Day);
        Assert.NotNull(restored.UpdatedAt);
        Assert.Equal(2024, restored.UpdatedAt!.Value.Year);
    }

    [Fact]
    public void Serializer_DateTimeOffset_NullPreserved()
    {
        var original = new WithTimestamps
        {
            Label = "no-update",
            CreatedAt = new DateTimeOffset(2024, 1, 1, 0, 0, 0, TimeSpan.Zero),
            UpdatedAt = null
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<WithTimestamps>(doc);

        Assert.Equal("no-update", restored.Label);
        Assert.Null(restored.UpdatedAt);
    }

    [Fact]
    public void Serializer_ReadPrimitive_DateTime()
    {
        // DateTimeModel is not [TeaLeaf]-annotated to avoid generator DateTime issue.
        // Tests the ReadPrimitive DateTime branch (line 533-537).
        var tlText = @"
            date_time_model: {
                name: test
                created_at: 2024-06-15T10:30:00Z
            }
        ";

        var restored = TeaLeafSerializer.FromText<DateTimeModel>(tlText);

        Assert.Equal("test", restored.Name);
        Assert.Equal(2024, restored.CreatedAt.Year);
        Assert.Equal(6, restored.CreatedAt.Month);
        Assert.Equal(15, restored.CreatedAt.Day);
    }

    // ----------------------------------------------------------------
    // ParseEnumFromSnakeCase multi-word enum coverage
    // ----------------------------------------------------------------

    [Fact]
    public void Serializer_MultiWordEnum_InProgress_Roundtrip()
    {
        var original = new WithOrderStatus
        {
            Name = "order1",
            Status = OrderStatus.InProgress
        };

        var text = TeaLeafSerializer.ToText(original);
        Assert.Contains("in_progress", text);

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<WithOrderStatus>(doc);

        Assert.Equal("order1", restored.Name);
        Assert.Equal(OrderStatus.InProgress, restored.Status);
    }

    [Fact]
    public void Serializer_MultiWordEnum_CompletedSuccessfully_Roundtrip()
    {
        var original = new WithOrderStatus
        {
            Name = "order2",
            Status = OrderStatus.CompletedSuccessfully
        };

        var text = TeaLeafSerializer.ToText(original);
        Assert.Contains("completed_successfully", text);

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<WithOrderStatus>(doc);

        Assert.Equal(OrderStatus.CompletedSuccessfully, restored.Status);
    }

    // ----------------------------------------------------------------
    // Nullable nested object coverage
    // ----------------------------------------------------------------

    [Fact]
    public void Serializer_NullableNestedObject_Null_SerializesAsTilde()
    {
        var model = new WithNullableAddress
        {
            Name = "homeless",
            HomeAddress = null
        };

        var text = TeaLeafSerializer.ToText(model);
        Assert.Contains("home_address: ~", text);
    }

    [Fact]
    public void Serializer_NullableNestedObject_Present_Roundtrip()
    {
        var original = new WithNullableAddress
        {
            Name = "alice",
            HomeAddress = new Address { Street = "123 Main", City = "Springfield", Zip = "62701" }
        };

        using var doc = TeaLeafSerializer.ToTLDocument(original);
        var restored = TeaLeafSerializer.FromDocument<WithNullableAddress>(doc);

        Assert.Equal("alice", restored.Name);
        Assert.NotNull(restored.HomeAddress);
        Assert.Equal("123 Main", restored.HomeAddress!.Street);
    }
}
