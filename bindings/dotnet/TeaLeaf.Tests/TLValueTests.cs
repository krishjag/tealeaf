using Xunit;

namespace TeaLeaf.Tests;

public class TLValueTests
{
    [Fact]
    public void ToObject_Null_ReturnsNull()
    {
        using var doc = TLDocument.Parse("value: ~");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Null, value.Type);
        Assert.Null(value.ToObject());
    }

    [Fact]
    public void ToObject_Bool_ReturnsBool()
    {
        using var doc = TLDocument.Parse("value: true");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<bool>(obj);
        Assert.True((bool)obj!);
    }

    [Fact]
    public void ToObject_Int_ReturnsLong()
    {
        using var doc = TLDocument.Parse("value: 12345");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<long>(obj);
        Assert.Equal(12345L, obj);
    }

    [Fact]
    public void ToObject_Float_ReturnsDouble()
    {
        using var doc = TLDocument.Parse("value: 3.14");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<double>(obj);
        Assert.Equal(3.14, (double)obj!, 2);
    }

    [Fact]
    public void ToObject_String_ReturnsString()
    {
        using var doc = TLDocument.Parse("value: hello");
        using var value = doc["value"];

        var obj = value?.ToObject();
        Assert.IsType<string>(obj);
        Assert.Equal("hello", obj);
    }

    [Fact]
    public void ToObject_Array_ReturnsArray()
    {
        using var doc = TLDocument.Parse("value: [1, 2, 3]");
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
        using var doc = TLDocument.Parse("value: {a: 1, b: 2}");
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
        using var doc = TLDocument.Parse(@"
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
        using var doc = TLDocument.Parse("value: {a: 1, b: 2, c: 3}");
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
        using var doc = TLDocument.Parse("value: [1, 2]");
        using var value = doc["value"];

        Assert.Null(value?[100]);
    }

    [Fact]
    public void AsRefName_OnReference_ReturnsName()
    {
        // Parse a document with a reference
        using var doc = TLDocument.Parse(@"
            base: {host: localhost}
            config: !base
        ");
        using var value = doc["config"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Ref, value.Type);
        Assert.Equal("base", value.AsRefName());
    }

    [Fact]
    public void AsRefName_OnNonRef_ReturnsNull()
    {
        using var doc = TLDocument.Parse("value: 123");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Null(value.AsRefName());
    }

    [Fact]
    public void AsTagged_OnTaggedValue_ReturnsTagAndValue()
    {
        using var doc = TLDocument.Parse("status: :ok 200");
        using var value = doc["status"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Tagged, value.Type);
        Assert.Equal("ok", value.AsTagName());

        using var inner = value.AsTagValue();
        Assert.NotNull(inner);
        Assert.Equal(200L, inner.AsInt());
    }

    [Fact]
    public void AsTagged_OnNonTagged_ReturnsNull()
    {
        using var doc = TLDocument.Parse("value: 123");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Null(value.AsTagName());
        Assert.Null(value.AsTagValue());
    }

    [Fact]
    public void AsBytes_OnNonBytes_ReturnsNull()
    {
        using var doc = TLDocument.Parse("value: hello");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Null(value.AsBytes());
    }

    [Fact]
    public void MapAccessors_OnNonMap_ReturnsZeroOrNull()
    {
        using var doc = TLDocument.Parse("value: [1, 2, 3]");
        using var value = doc["value"];

        Assert.NotNull(value);
        Assert.Equal(0, value.MapLength);
        Assert.Null(value.GetMapKey(0));
        Assert.Null(value.GetMapValue(0));
    }

    [Fact]
    public void ToObject_Ref_ReturnsDictionaryWithRefKey()
    {
        using var doc = TLDocument.Parse(@"
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
        using var doc = TLDocument.Parse("status: :error 404");
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
        using var doc = TLDocument.Parse(@"
            lookup: @map {
                1: one,
                2: two,
                3: three
            }
        ");
        using var value = doc["lookup"];

        Assert.NotNull(value);
        Assert.Equal(TLType.Map, value.Type);
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
        using var doc = TLDocument.Parse(@"
            lookup: @map {
                a: 1,
                b: 2
            }
        ");
        using var value = doc["lookup"];

        Assert.NotNull(value);
        var pairs = value.AsMap().ToList();
        Assert.Equal(2, pairs.Count);

        // Clean up the yielded TLValue instances
        foreach (var (k, v) in pairs)
        {
            k.Dispose();
            v.Dispose();
        }
    }

    [Fact]
    public void ToObject_Map_ReturnsKeyValuePairArray()
    {
        using var doc = TLDocument.Parse(@"
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

    // =========================================================================
    // GetRequired() extension methods
    // =========================================================================

    [Fact]
    public void GetRequired_Document_ReturnsValue()
    {
        using var doc = TLDocument.Parse("name: alice");
        using var value = doc.GetRequired("name");
        Assert.Equal("alice", value.AsString());
    }

    [Fact]
    public void GetRequired_Document_ThrowsOnMissingKey()
    {
        using var doc = TLDocument.Parse("name: alice");
        var ex = Assert.Throws<KeyNotFoundException>(() => doc.GetRequired("missing"));
        Assert.Contains("missing", ex.Message);
    }

    [Fact]
    public void GetRequired_ObjectField_ReturnsValue()
    {
        using var doc = TLDocument.Parse("user: {name: alice, age: 30}");
        using var user = doc.GetRequired("user");
        using var name = user.GetRequired("name");
        Assert.Equal("alice", name.AsString());
    }

    [Fact]
    public void GetRequired_ObjectField_ThrowsOnMissingKey()
    {
        using var doc = TLDocument.Parse("user: {name: alice}");
        using var user = doc.GetRequired("user");
        var ex = Assert.Throws<KeyNotFoundException>(() => user.GetRequired("email"));
        Assert.Contains("email", ex.Message);
    }

    [Fact]
    public void GetRequired_ArrayElement_ReturnsValue()
    {
        using var doc = TLDocument.Parse("items: [10, 20, 30]");
        using var items = doc.GetRequired("items");
        using var second = items.GetRequired(1);
        Assert.Equal(20L, second.AsInt());
    }

    [Fact]
    public void GetRequired_ArrayElement_ThrowsOnOutOfBounds()
    {
        using var doc = TLDocument.Parse("items: [10, 20]");
        using var items = doc.GetRequired("items");
        Assert.Throws<IndexOutOfRangeException>(() => items.GetRequired(99));
    }

    // ------------------------------------------------------------------
    // Object field iteration
    // ------------------------------------------------------------------

    [Fact]
    public void ObjectFieldCount_ReturnsCorrectCount()
    {
        using var doc = TLDocument.Parse("value: {a: 1, b: 2, c: 3}");
        using var value = doc["value"];
        Assert.NotNull(value);
        Assert.Equal(3, value.ObjectFieldCount);
    }

    [Fact]
    public void ObjectFieldCount_EmptyObject_ReturnsZero()
    {
        using var doc = TLDocument.Parse("value: {}");
        using var value = doc["value"];
        Assert.NotNull(value);
        Assert.Equal(0, value.ObjectFieldCount);
    }

    [Fact]
    public void ObjectFieldCount_NotObject_ReturnsZero()
    {
        using var doc = TLDocument.Parse("value: hello");
        using var value = doc["value"];
        Assert.NotNull(value);
        Assert.Equal(0, value.ObjectFieldCount);
    }

    [Fact]
    public void GetObjectKeyAt_ReturnsKeysInOrder()
    {
        using var doc = TLDocument.Parse("value: {name: alice, age: 30, active: true}");
        using var value = doc["value"];
        Assert.NotNull(value);

        Assert.Equal("name", value.GetObjectKeyAt(0));
        Assert.Equal("age", value.GetObjectKeyAt(1));
        Assert.Equal("active", value.GetObjectKeyAt(2));
    }

    [Fact]
    public void GetObjectValueAt_ReturnsValuesInOrder()
    {
        using var doc = TLDocument.Parse("value: {name: alice, age: 30, active: true}");
        using var value = doc["value"];
        Assert.NotNull(value);

        using var val0 = value.GetObjectValueAt(0);
        Assert.Equal("alice", val0?.AsString());

        using var val1 = value.GetObjectValueAt(1);
        Assert.Equal(30, val1?.AsInt());

        using var val2 = value.GetObjectValueAt(2);
        Assert.True(val2?.AsBool());
    }

    [Fact]
    public void GetObjectKeyAt_OutOfBounds_ReturnsNull()
    {
        using var doc = TLDocument.Parse("value: {a: 1}");
        using var value = doc["value"];
        Assert.NotNull(value);

        Assert.Null(value.GetObjectKeyAt(99));
        Assert.Null(value.GetObjectKeyAt(-1));
    }

    [Fact]
    public void GetObjectValueAt_OutOfBounds_ReturnsNull()
    {
        using var doc = TLDocument.Parse("value: {a: 1}");
        using var value = doc["value"];
        Assert.NotNull(value);

        Assert.Null(value.GetObjectValueAt(99));
        Assert.Null(value.GetObjectValueAt(-1));
    }

    [Fact]
    public void AsObject_EnumeratesAllPairs()
    {
        using var doc = TLDocument.Parse("value: {name: alice, age: 30}");
        using var value = doc["value"];
        Assert.NotNull(value);

        var pairs = value.AsObject().ToList();
        Assert.Equal(2, pairs.Count);

        Assert.Equal("name", pairs[0].Key);
        Assert.Equal("alice", pairs[0].Value.AsString());

        Assert.Equal("age", pairs[1].Key);
        Assert.Equal(30, pairs[1].Value.AsInt());

        // Clean up
        foreach (var (_, v) in pairs)
            v.Dispose();
    }

    [Fact]
    public void AsObject_NotObject_YieldsEmpty()
    {
        using var doc = TLDocument.Parse("value: hello");
        using var value = doc["value"];
        Assert.NotNull(value);

        var pairs = value.AsObject().ToList();
        Assert.Empty(pairs);
    }

    // ------------------------------------------------------------------
    // GetPath — dot-path navigation
    // ------------------------------------------------------------------

    private const string NestedDoc = """
        order: {
            order_id: ORD-001
            customer: {
                name: Alice
                email: "alice@example.com"
                address: {
                    city: Seattle
                    state: WA
                    zip: "98101"
                }
            }
            items: [
                {
                    product: {
                        name: Headphones
                        price: {
                            base_price: 349.99
                            currency: USD
                        }
                    }
                    quantity: 1
                }
                {
                    product: {
                        name: Keyboard
                        price: {
                            base_price: 159.0
                            currency: USD
                        }
                    }
                    quantity: 2
                }
            ]
            status: delivered
        }
        """;

    [Fact]
    public void GetPath_SingleLevel()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.order_id");
        Assert.NotNull(val);
        Assert.Equal("ORD-001", val.AsString());
    }

    [Fact]
    public void GetPath_TwoLevels()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.customer.name");
        Assert.NotNull(val);
        Assert.Equal("Alice", val.AsString());
    }

    [Fact]
    public void GetPath_ThreeLevels()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.customer.address.city");
        Assert.NotNull(val);
        Assert.Equal("Seattle", val.AsString());
    }

    [Fact]
    public void GetPath_ArrayIndex()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.items[0].quantity");
        Assert.NotNull(val);
        Assert.Equal(1, val.AsInt());
    }

    [Fact]
    public void GetPath_ArrayIndexSecondElement()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.items[1].product.name");
        Assert.NotNull(val);
        Assert.Equal("Keyboard", val.AsString());
    }

    [Fact]
    public void GetPath_DeepNest_SixLevels()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.items[0].product.price.base_price");
        Assert.NotNull(val);
        Assert.True(Math.Abs(349.99 - val.AsFloat()!.Value) < 0.01);
    }

    [Fact]
    public void GetPath_MissingKey_ReturnsNull()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.customer.phone");
        Assert.Null(val);
    }

    [Fact]
    public void GetPath_MissingIntermediate_ReturnsNull()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.nonexistent.field");
        Assert.Null(val);
    }

    [Fact]
    public void GetPath_ArrayOutOfBounds_ReturnsNull()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order.items[99].product.name");
        Assert.Null(val);
    }

    [Fact]
    public void GetPath_DocumentLevel_JustKey()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var val = doc.GetPath("order");
        Assert.NotNull(val);
        Assert.Equal(TLType.Object, val.Type);
    }

    [Fact]
    public void GetPath_OnValue_RelativePath()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        using var order = doc["order"];
        Assert.NotNull(order);

        using var val = order.GetPath("items[1].product.price.currency");
        Assert.NotNull(val);
        Assert.Equal("USD", val.AsString());
    }

    [Fact]
    public void GetPath_EmptyPath_ReturnsNull()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        Assert.Null(doc.GetPath(""));
        Assert.Null(doc.GetPath(null!));
    }

    [Fact]
    public void GetPath_NonexistentRoot_ReturnsNull()
    {
        using var doc = TLDocument.Parse(NestedDoc);
        Assert.Null(doc.GetPath("nosuchkey.field"));
    }
}
