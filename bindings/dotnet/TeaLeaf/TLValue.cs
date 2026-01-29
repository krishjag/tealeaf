using System.Collections;
using TeaLeaf.Native;

namespace TeaLeaf;

/// <summary>
/// Represents a value in a TeaLeaf document.
/// </summary>
public sealed class TLValue : IDisposable
{
    private IntPtr _handle;
    private bool _disposed;

    internal TLValue(IntPtr handle)
    {
        _handle = handle;
    }

    /// <summary>
    /// Gets the type of this value.
    /// </summary>
    public TLType Type
    {
        get
        {
            ThrowIfDisposed();
            return (TLType)NativeMethods.tl_value_type(_handle);
        }
    }

    /// <summary>
    /// Returns true if this value is null.
    /// </summary>
    public bool IsNull => Type == TLType.Null;

    /// <summary>
    /// Gets this value as a boolean. Returns null if not a boolean.
    /// </summary>
    public bool? AsBool()
    {
        ThrowIfDisposed();
        if (Type != TLType.Bool) return null;
        return NativeMethods.tl_value_as_bool(_handle);
    }

    /// <summary>
    /// Gets this value as a long integer. Returns null if not an integer.
    /// </summary>
    public long? AsInt()
    {
        ThrowIfDisposed();
        if (Type != TLType.Int) return null;
        return NativeMethods.tl_value_as_int(_handle);
    }

    /// <summary>
    /// Gets this value as an unsigned long integer. Returns null if not an unsigned integer.
    /// </summary>
    public ulong? AsUInt()
    {
        ThrowIfDisposed();
        if (Type != TLType.UInt) return null;
        return NativeMethods.tl_value_as_uint(_handle);
    }

    /// <summary>
    /// Gets this value as a double. Returns null if not a float.
    /// </summary>
    public double? AsFloat()
    {
        ThrowIfDisposed();
        if (Type != TLType.Float) return null;
        return NativeMethods.tl_value_as_float(_handle);
    }

    /// <summary>
    /// Gets this value as a string. Returns null if not a string.
    /// </summary>
    public string? AsString()
    {
        ThrowIfDisposed();
        if (Type != TLType.String) return null;
        var ptr = NativeMethods.tl_value_as_string(_handle);
        return NativeMethods.PtrToStringAndFree(ptr);
    }

    /// <summary>
    /// Gets this value as a timestamp (Unix milliseconds). Returns null if not a timestamp.
    /// </summary>
    public long? AsTimestamp()
    {
        ThrowIfDisposed();
        if (Type != TLType.Timestamp) return null;
        return NativeMethods.tl_value_as_timestamp(_handle);
    }

    /// <summary>
    /// Gets this value as a DateTimeOffset. Returns null if not a timestamp.
    /// </summary>
    public DateTimeOffset? AsDateTime()
    {
        var ts = AsTimestamp();
        if (ts == null) return null;
        return DateTimeOffset.FromUnixTimeMilliseconds(ts.Value);
    }

    /// <summary>
    /// Gets this value as a byte array. Returns null if not bytes.
    /// </summary>
    public byte[]? AsBytes()
    {
        ThrowIfDisposed();
        if (Type != TLType.Bytes) return null;
        var len = (int)NativeMethods.tl_value_bytes_len(_handle);
        if (len == 0) return Array.Empty<byte>();
        var ptr = NativeMethods.tl_value_bytes_data(_handle);
        if (ptr == IntPtr.Zero) return Array.Empty<byte>();
        var result = new byte[len];
        System.Runtime.InteropServices.Marshal.Copy(ptr, result, 0, len);
        return result;
    }

    /// <summary>
    /// Gets the reference name if this is a Ref value. Returns null otherwise.
    /// </summary>
    public string? AsRefName()
    {
        ThrowIfDisposed();
        if (Type != TLType.Ref) return null;
        var ptr = NativeMethods.tl_value_ref_name(_handle);
        return NativeMethods.PtrToStringAndFree(ptr);
    }

    /// <summary>
    /// Gets the tag name if this is a Tagged value. Returns null otherwise.
    /// </summary>
    public string? AsTagName()
    {
        ThrowIfDisposed();
        if (Type != TLType.Tagged) return null;
        var ptr = NativeMethods.tl_value_tag_name(_handle);
        return NativeMethods.PtrToStringAndFree(ptr);
    }

    /// <summary>
    /// Gets the inner value if this is a Tagged value. Returns null otherwise.
    /// </summary>
    public TLValue? AsTagValue()
    {
        ThrowIfDisposed();
        if (Type != TLType.Tagged) return null;
        var ptr = NativeMethods.tl_value_tag_value(_handle);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Gets the length of this map value. Returns 0 if not a map.
    /// </summary>
    public int MapLength
    {
        get
        {
            ThrowIfDisposed();
            if (Type != TLType.Map) return 0;
            return (int)NativeMethods.tl_value_map_len(_handle);
        }
    }

    /// <summary>
    /// Gets the key at the specified index from this map value. Returns null if not a map or out of bounds.
    /// </summary>
    public TLValue? GetMapKey(int index)
    {
        ThrowIfDisposed();
        if (Type != TLType.Map || index < 0) return null;
        var ptr = NativeMethods.tl_value_map_get_key(_handle, (nuint)index);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Gets the value at the specified index from this map value. Returns null if not a map or out of bounds.
    /// </summary>
    public TLValue? GetMapValue(int index)
    {
        ThrowIfDisposed();
        if (Type != TLType.Map || index < 0) return null;
        var ptr = NativeMethods.tl_value_map_get_value(_handle, (nuint)index);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Gets all key-value pairs from this map value.
    /// </summary>
    public IEnumerable<(TLValue Key, TLValue Value)> AsMap()
    {
        ThrowIfDisposed();
        if (Type != TLType.Map)
            yield break;

        int len = MapLength;
        for (int i = 0; i < len; i++)
        {
            var key = GetMapKey(i);
            var value = GetMapValue(i);
            if (key != null && value != null)
                yield return (key, value);
        }
    }

    /// <summary>
    /// Gets the length of this array value. Returns 0 if not an array.
    /// </summary>
    public int ArrayLength
    {
        get
        {
            ThrowIfDisposed();
            if (Type != TLType.Array) return 0;
            return (int)NativeMethods.tl_value_array_len(_handle);
        }
    }

    /// <summary>
    /// Gets an element from this array value by index. Returns null if not an array or out of bounds.
    /// </summary>
    public TLValue? GetArrayElement(int index)
    {
        ThrowIfDisposed();
        if (Type != TLType.Array || index < 0) return null;
        var ptr = NativeMethods.tl_value_array_get(_handle, (nuint)index);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Gets all elements from this array value.
    /// </summary>
    public IEnumerable<TLValue> AsArray()
    {
        ThrowIfDisposed();
        if (Type != TLType.Array)
            yield break;

        int len = ArrayLength;
        for (int i = 0; i < len; i++)
        {
            var elem = GetArrayElement(i);
            if (elem != null)
                yield return elem;
        }
    }

    /// <summary>
    /// Gets a field from this object value by key. Returns null if not an object or key not found.
    /// </summary>
    public TLValue? GetField(string key)
    {
        ThrowIfDisposed();
        if (Type != TLType.Object) return null;
        var ptr = NativeMethods.tl_value_object_get(_handle, key);
        return ptr == IntPtr.Zero ? null : new TLValue(ptr);
    }

    /// <summary>
    /// Gets all keys from this object value.
    /// </summary>
    public string[] GetObjectKeys()
    {
        ThrowIfDisposed();
        if (Type != TLType.Object) return Array.Empty<string>();
        var ptr = NativeMethods.tl_value_object_keys(_handle);
        return NativeMethods.PtrToStringArrayAndFree(ptr);
    }

    /// <summary>
    /// Gets all keys from this object value.
    /// </summary>
    public string[] ObjectKeys => GetObjectKeys();

    /// <summary>
    /// Indexer for array access.
    /// </summary>
    public TLValue? this[int index] => GetArrayElement(index);

    /// <summary>
    /// Indexer for object field access.
    /// </summary>
    public TLValue? this[string key] => GetField(key);

    /// <summary>
    /// Converts this value to its .NET equivalent.
    /// </summary>
    public object? ToObject()
    {
        ThrowIfDisposed();

        return Type switch
        {
            TLType.Null => null,
            TLType.Bool => AsBool(),
            TLType.Int => AsInt(),
            TLType.UInt => AsUInt(),
            TLType.Float => AsFloat(),
            TLType.String => AsString(),
            TLType.Bytes => AsBytes(),
            TLType.Timestamp => AsDateTime(),
            TLType.Array => AsArray().Select(v => v.ToObject()).ToArray(),
            TLType.Object => GetObjectKeys().ToDictionary(k => k, k => GetField(k)?.ToObject()),
            TLType.Map => AsMap().Select(kv => new KeyValuePair<object?, object?>(kv.Key.ToObject(), kv.Value.ToObject())).ToArray(),
            TLType.Ref => new Dictionary<string, object?> { ["$ref"] = AsRefName() },
            TLType.Tagged => new Dictionary<string, object?> { ["$tag"] = AsTagName(), ["$value"] = AsTagValue()?.ToObject() },
            _ => null
        };
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(TLValue));
    }

    public void Dispose()
    {
        if (!_disposed && _handle != IntPtr.Zero)
        {
            NativeMethods.tl_value_free(_handle);
            _handle = IntPtr.Zero;
            _disposed = true;
        }
    }
}

/// <summary>
/// The type of a TeaLeaf value.
/// </summary>
public enum TLType
{
    Null = 0,
    Bool = 1,
    Int = 2,
    UInt = 3,
    Float = 4,
    String = 5,
    Bytes = 6,
    Array = 7,
    Object = 8,
    Map = 9,
    Ref = 10,
    Tagged = 11,
    Timestamp = 12,
}
