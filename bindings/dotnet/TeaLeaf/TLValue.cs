using System.Collections;
using Pax.Native;

namespace Pax;

/// <summary>
/// Represents a value in a Pax document.
/// </summary>
public sealed class PaxValue : IDisposable
{
    private IntPtr _handle;
    private bool _disposed;

    internal PaxValue(IntPtr handle)
    {
        _handle = handle;
    }

    /// <summary>
    /// Gets the type of this value.
    /// </summary>
    public PaxType Type
    {
        get
        {
            ThrowIfDisposed();
            return (PaxType)NativeMethods.pax_value_type(_handle);
        }
    }

    /// <summary>
    /// Returns true if this value is null.
    /// </summary>
    public bool IsNull => Type == PaxType.Null;

    /// <summary>
    /// Gets this value as a boolean. Returns null if not a boolean.
    /// </summary>
    public bool? AsBool()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Bool) return null;
        return NativeMethods.pax_value_as_bool(_handle);
    }

    /// <summary>
    /// Gets this value as a long integer. Returns null if not an integer.
    /// </summary>
    public long? AsInt()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Int) return null;
        return NativeMethods.pax_value_as_int(_handle);
    }

    /// <summary>
    /// Gets this value as an unsigned long integer. Returns null if not an unsigned integer.
    /// </summary>
    public ulong? AsUInt()
    {
        ThrowIfDisposed();
        if (Type != PaxType.UInt) return null;
        return NativeMethods.pax_value_as_uint(_handle);
    }

    /// <summary>
    /// Gets this value as a double. Returns null if not a float.
    /// </summary>
    public double? AsFloat()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Float) return null;
        return NativeMethods.pax_value_as_float(_handle);
    }

    /// <summary>
    /// Gets this value as a string. Returns null if not a string.
    /// </summary>
    public string? AsString()
    {
        ThrowIfDisposed();
        if (Type != PaxType.String) return null;
        var ptr = NativeMethods.pax_value_as_string(_handle);
        return NativeMethods.PtrToStringAndFree(ptr);
    }

    /// <summary>
    /// Gets this value as a timestamp (Unix milliseconds). Returns null if not a timestamp.
    /// </summary>
    public long? AsTimestamp()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Timestamp) return null;
        return NativeMethods.pax_value_as_timestamp(_handle);
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
        if (Type != PaxType.Bytes) return null;
        var len = (int)NativeMethods.pax_value_bytes_len(_handle);
        if (len == 0) return Array.Empty<byte>();
        var ptr = NativeMethods.pax_value_bytes_data(_handle);
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
        if (Type != PaxType.Ref) return null;
        var ptr = NativeMethods.pax_value_ref_name(_handle);
        return NativeMethods.PtrToStringAndFree(ptr);
    }

    /// <summary>
    /// Gets the tag name if this is a Tagged value. Returns null otherwise.
    /// </summary>
    public string? AsTagName()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Tagged) return null;
        var ptr = NativeMethods.pax_value_tag_name(_handle);
        return NativeMethods.PtrToStringAndFree(ptr);
    }

    /// <summary>
    /// Gets the inner value if this is a Tagged value. Returns null otherwise.
    /// </summary>
    public PaxValue? AsTagValue()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Tagged) return null;
        var ptr = NativeMethods.pax_value_tag_value(_handle);
        return ptr == IntPtr.Zero ? null : new PaxValue(ptr);
    }

    /// <summary>
    /// Gets the length of this map value. Returns 0 if not a map.
    /// </summary>
    public int MapLength
    {
        get
        {
            ThrowIfDisposed();
            if (Type != PaxType.Map) return 0;
            return (int)NativeMethods.pax_value_map_len(_handle);
        }
    }

    /// <summary>
    /// Gets the key at the specified index from this map value. Returns null if not a map or out of bounds.
    /// </summary>
    public PaxValue? GetMapKey(int index)
    {
        ThrowIfDisposed();
        if (Type != PaxType.Map || index < 0) return null;
        var ptr = NativeMethods.pax_value_map_get_key(_handle, (nuint)index);
        return ptr == IntPtr.Zero ? null : new PaxValue(ptr);
    }

    /// <summary>
    /// Gets the value at the specified index from this map value. Returns null if not a map or out of bounds.
    /// </summary>
    public PaxValue? GetMapValue(int index)
    {
        ThrowIfDisposed();
        if (Type != PaxType.Map || index < 0) return null;
        var ptr = NativeMethods.pax_value_map_get_value(_handle, (nuint)index);
        return ptr == IntPtr.Zero ? null : new PaxValue(ptr);
    }

    /// <summary>
    /// Gets all key-value pairs from this map value.
    /// </summary>
    public IEnumerable<(PaxValue Key, PaxValue Value)> AsMap()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Map)
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
            if (Type != PaxType.Array) return 0;
            return (int)NativeMethods.pax_value_array_len(_handle);
        }
    }

    /// <summary>
    /// Gets an element from this array value by index. Returns null if not an array or out of bounds.
    /// </summary>
    public PaxValue? GetArrayElement(int index)
    {
        ThrowIfDisposed();
        if (Type != PaxType.Array || index < 0) return null;
        var ptr = NativeMethods.pax_value_array_get(_handle, (nuint)index);
        return ptr == IntPtr.Zero ? null : new PaxValue(ptr);
    }

    /// <summary>
    /// Gets all elements from this array value.
    /// </summary>
    public IEnumerable<PaxValue> AsArray()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Array)
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
    public PaxValue? GetField(string key)
    {
        ThrowIfDisposed();
        if (Type != PaxType.Object) return null;
        var ptr = NativeMethods.pax_value_object_get(_handle, key);
        return ptr == IntPtr.Zero ? null : new PaxValue(ptr);
    }

    /// <summary>
    /// Gets all keys from this object value.
    /// </summary>
    public string[] GetObjectKeys()
    {
        ThrowIfDisposed();
        if (Type != PaxType.Object) return Array.Empty<string>();
        var ptr = NativeMethods.pax_value_object_keys(_handle);
        return NativeMethods.PtrToStringArrayAndFree(ptr);
    }

    /// <summary>
    /// Gets all keys from this object value.
    /// </summary>
    public string[] ObjectKeys => GetObjectKeys();

    /// <summary>
    /// Indexer for array access.
    /// </summary>
    public PaxValue? this[int index] => GetArrayElement(index);

    /// <summary>
    /// Indexer for object field access.
    /// </summary>
    public PaxValue? this[string key] => GetField(key);

    /// <summary>
    /// Converts this value to its .NET equivalent.
    /// </summary>
    public object? ToObject()
    {
        ThrowIfDisposed();

        return Type switch
        {
            PaxType.Null => null,
            PaxType.Bool => AsBool(),
            PaxType.Int => AsInt(),
            PaxType.UInt => AsUInt(),
            PaxType.Float => AsFloat(),
            PaxType.String => AsString(),
            PaxType.Bytes => AsBytes(),
            PaxType.Timestamp => AsDateTime(),
            PaxType.Array => AsArray().Select(v => v.ToObject()).ToArray(),
            PaxType.Object => GetObjectKeys().ToDictionary(k => k, k => GetField(k)?.ToObject()),
            PaxType.Map => AsMap().Select(kv => new KeyValuePair<object?, object?>(kv.Key.ToObject(), kv.Value.ToObject())).ToArray(),
            PaxType.Ref => new Dictionary<string, object?> { ["$ref"] = AsRefName() },
            PaxType.Tagged => new Dictionary<string, object?> { ["$tag"] = AsTagName(), ["$value"] = AsTagValue()?.ToObject() },
            _ => null
        };
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(PaxValue));
    }

    public void Dispose()
    {
        if (!_disposed && _handle != IntPtr.Zero)
        {
            NativeMethods.pax_value_free(_handle);
            _handle = IntPtr.Zero;
            _disposed = true;
        }
    }
}

/// <summary>
/// The type of a Pax value.
/// </summary>
public enum PaxType
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
