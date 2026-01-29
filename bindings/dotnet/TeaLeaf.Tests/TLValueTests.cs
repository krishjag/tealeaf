using Xunit;

namespace Pax.Tests;

public class PaxValueTests
{
    [Fact]
    public void ToObject_Null_ReturnsNull()
    {
        using var doc = PaxDocument.Parse("value: ~");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Equal(PaxType.Null, value.Type);
        Assert.Null(value.ToObject());
    }

    [Fact]
    public void ToObject_Bool_ReturnsBool()
    {
        using var doc = PaxDocument.Parse("value: true");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<bool>(obj);
        Assert.True((bool)obj!);
    }

    [Fact]
    public void ToObject_Int_ReturnsLong()
    {
        using var doc = PaxDocument.Parse("value: 12345");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<long>(obj);
        Assert.Equal(12345L, obj);
    }

    [Fact]
    public void ToObject_Float_ReturnsDouble()
    {
        using var doc = PaxDocument.Parse("value: 3.14");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<double>(obj);
        Assert.Equal(3.14, (double)obj!, 2);
    }

    [Fact]
    public void ToObject_String_ReturnsString()
    {
        using var doc = PaxDocument.Parse("value: hello");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<string>(obj);
        Assert.Equal("hello", obj);
    }

    [Fact]
    public void ToObject_Array_ReturnsArray()
    {
        using var doc = PaxDocument.Parse("value: [1, 2, 3]");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<object?[]>(obj);
        var arr = (object?[])obj!;
        Assert.Equal(3, arr.Length);
        Assert.Equal(1L, arr[0]);
    }

    [Fact]
    public void ToObject_Object_ReturnsDictionary()
    {
        using var doc = PaxDocument.Parse("value: {a: 1, b: 2}");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<Dictionary<string, object?>>(obj);
        var dict = (Dictionary<string, object?>)obj!;
        Assert.Equal(2, dict.Count);
        Assert.Equal(1L, dict["a"]);
    }

    [Fact]
    public void NestedObject_AccessWorks()
    {
        using var doc = PaxDocument.Parse(@"
            user: {
                name: alice,
                address: {
                    city: Berlin,
                    zip: 10115
                }
            }
        ");
        using var user = doc["user"];
        using var address = user?["address"];
        using var city = address?["city"];

        Assert.Equal("Berlin", city?.AsString());
    }

    [Fact]
    public void ObjectKeys_ReturnsAllKeys()
    {
        using var doc = PaxDocument.Parse("value: {a: 1, b: 2, c: 3}");
        using var value = doc["value"];

        var keys = value?.ObjectKeys;
        Assert.NotNull(keys);
        Assert.Equal(3, keys!.Length);
        Assert.Contains("a", keys);
        Assert.Contains("b", keys);
        Assert.Contains("c", keys);
    }

    [Fact]
    public void ArrayAccess_OutOfBounds_ReturnsNull()
    {
        using var doc = PaxDocument.Parse("value: [1, 2]");
        using var value = doc["value"];

        Assert.Null(value?[100]);
    }

    [Fact]
    public void AsRefName_OnReference_ReturnsName()
    {
        // Parse a document with a reference
        using var doc = PaxDocument.Parse(@"
            base: {host: localhost}
            config: !base
        ");
        using var value = doc["config"];

        Assert.NotNull(value);
        Assert.Equal(PaxType.Ref, value.Type);
        Assert.Equal("base", value.AsRefName());
    }

    [Fact]
    public void AsRefName_OnNonRef_ReturnsNull()
    {
        using var doc = PaxDocument.Parse("value: 123");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Null(value.AsRefName());
    }

    [Fact]
    public void AsTagged_OnTaggedValue_ReturnsTagAndValue()
    {
        using var doc = PaxDocument.Parse("status: :ok 200");
        using var value = doc["status"];

        Assert.NotNull(value);
        Assert.Equal(PaxType.Tagged, value.Type);
        Assert.Equal("ok", value.AsTagName());

        using var inner = value.AsTagValue();
        Assert.NotNull(inner);
        Assert.Equal(200L, inner.AsInt());
    }

    [Fact]
    public void AsTagged_OnNonTagged_ReturnsNull()
    {
        using var doc = PaxDocument.Parse("value: 123");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Null(value.AsTagName());
        Assert.Null(value.AsTagValue());
    }

    [Fact]
    public void AsBytes_OnNonBytes_ReturnsNull()
    {
        using var doc = PaxDocument.Parse("value: hello");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Null(value.AsBytes());
    }

    [Fact]
    public void MapAccessors_OnNonMap_ReturnsZeroOrNull()
    {
        using var doc = PaxDocument.Parse("value: [1, 2, 3]");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Equal(0, value.MapLength);
        Assert.Null(value.GetMapKey(0));
        Assert.Null(value.GetMapValue(0));
    }

    [Fact]
    public void ToObject_Ref_ReturnsDictionaryWithRefKey()
    {
        using var doc = PaxDocument.Parse(@"
            base: {x: 1}
            ref: !base
        ");
        using var value = doc["ref"];

        var obj = value?.ToObject();
        Assert.IsType<Dictionary<string, object?>>(obj);
        var dict = (Dictionary<string, object?>)obj!;
        Assert.True(dict.ContainsKey("$ref"));
        Assert.Equal("base", dict["$ref"]);
    }

    [Fact]
    public void ToObject_Tagged_ReturnsDictionaryWithTagAndValue()
    {
        using var doc = PaxDocument.Parse("status: :error 404");
        using var value = doc["status"];

        var obj = value?.ToObject();
        Assert.IsType<Dictionary<string, object?>>(obj);
        var dict = (Dictionary<string, object?>)obj!;
        Assert.True(dict.ContainsKey("$tag"));
        Assert.True(dict.ContainsKey("$value"));
        Assert.Equal("error", dict["$tag"]);
        Assert.Equal(404L, dict["$value"]);
    }

    [Fact]
    public void Map_ParseAndAccess_Works()
    {
        // Parse a document with a @map
        using var doc = PaxDocument.Parse(@"
            lookup: @map {
                1: one,
                2: two,
                3: three
            }
        ");
        using var value = doc["lookup"];

        Assert.NotNull(value);
        Assert.Equal(PaxType.Map, value.Type);
        Assert.Equal(3, value.MapLength);

        // Access first entry
        using var key0 = value.GetMapKey(0);
        using var val0 = value.GetMapValue(0);
        Assert.NotNull(key0);
        Assert.NotNull(val0);
        Assert.Equal(1L, key0.AsInt());
        Assert.Equal("one", val0.AsString());
    }

    [Fact]
    public void Map_AsMap_EnumeratesAllPairs()
    {
        using var doc = PaxDocument.Parse(@"
            lookup: @map {
                a: 1,
                b: 2
            }
        ");
        using var value = doc["lookup"];

        Assert.NotNull(value);
        var pairs = value.AsMap().ToList();
        Assert.Equal(2, pairs.Count);

        // Clean up the yielded PaxValue instances
        foreach (var (k, v) in pairs)
        {
            k.Dispose();
            v.Dispose();
        }
    }

    [Fact]
    public void ToObject_Map_ReturnsKeyValuePairArray()
    {
        using var doc = PaxDocument.Parse(@"
            lookup: @map {
                1: one,
                2: two
            }
        ");
        using var value = doc["lookup"];

        var obj = value?.ToObject();
        Assert.IsType<KeyValuePair<object?, object?>[]>(obj);
        var arr = (KeyValuePair<object?, object?>[])obj!;
        Assert.Equal(2, arr.Length);
        Assert.Equal(1L, arr[0].Key);
        Assert.Equal("one", arr[0].Value);
    }
}
