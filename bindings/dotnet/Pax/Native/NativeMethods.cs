using System;
using System.Runtime.InteropServices;

namespace Pax.Native;

/// <summary>
/// P/Invoke declarations for the native Pax FFI library.
/// </summary>
internal static class NativeMethods
{
    private const string LibraryName = "pax_ffi";

    // ==========================================================================
    // Error Handling API
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_get_last_error();

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void pax_clear_error();

    /// <summary>
    /// Get the last error message from the native library and clear it.
    /// Returns null if no error.
    /// </summary>
    public static string? GetLastError()
    {
        var ptr = pax_get_last_error();
        if (ptr == IntPtr.Zero)
            return null;

        try
        {
            return Marshal.PtrToStringUTF8(ptr);
        }
        finally
        {
            pax_string_free(ptr);
        }
    }

    // ==========================================================================
    // Document API
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_parse(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string text);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_parse_file(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void pax_document_free(IntPtr doc);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_document_get(
        IntPtr doc,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string key);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_document_keys(IntPtr doc);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_document_to_text(IntPtr doc);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_document_to_text_data_only(IntPtr doc);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern PaxResult pax_document_compile(
        IntPtr doc,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path,
        [MarshalAs(UnmanagedType.I1)] bool compress);

    // ==========================================================================
    // JSON Conversion API
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_document_from_json(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string json);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_document_to_json(IntPtr doc);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_document_to_json_compact(IntPtr doc);

    // ==========================================================================
    // Value API
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern PaxValueType pax_value_type(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void pax_value_free(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    public static extern bool pax_value_as_bool(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern long pax_value_as_int(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern ulong pax_value_as_uint(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern double pax_value_as_float(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_as_string(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern long pax_value_as_timestamp(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern nuint pax_value_array_len(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_array_get(IntPtr value, nuint index);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_object_get(
        IntPtr value,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string key);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_object_keys(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern nuint pax_value_bytes_len(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_bytes_data(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_ref_name(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_tag_name(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_tag_value(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern nuint pax_value_map_len(IntPtr value);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_map_get_key(IntPtr value, nuint index);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_value_map_get_value(IntPtr value, nuint index);

    // ==========================================================================
    // Binary Reader API
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_reader_open(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_reader_open_mmap(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void pax_reader_free(IntPtr reader);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_reader_get(
        IntPtr reader,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string key);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_reader_keys(IntPtr reader);

    // ==========================================================================
    // Schema API (for dynamic typing support)
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern nuint pax_reader_schema_count(IntPtr reader);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_reader_schema_name(IntPtr reader, nuint index);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern nuint pax_reader_schema_field_count(IntPtr reader, nuint schemaIndex);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_reader_schema_field_name(IntPtr reader, nuint schemaIndex, nuint fieldIndex);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_reader_schema_field_type(IntPtr reader, nuint schemaIndex, nuint fieldIndex);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    public static extern bool pax_reader_schema_field_nullable(IntPtr reader, nuint schemaIndex, nuint fieldIndex);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    public static extern bool pax_reader_schema_field_is_array(IntPtr reader, nuint schemaIndex, nuint fieldIndex);

    // ==========================================================================
    // Memory Management
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void pax_string_free(IntPtr s);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void pax_string_array_free(IntPtr arr);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void pax_result_free(ref PaxResult result);

    // ==========================================================================
    // Version
    // ==========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr pax_version();

    // ==========================================================================
    // Helper methods
    // ==========================================================================

    /// <summary>
    /// Convert a native string pointer to a managed string and free the native memory.
    /// </summary>
    public static string? PtrToStringAndFree(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
            return null;

        try
        {
            return Marshal.PtrToStringUTF8(ptr);
        }
        finally
        {
            pax_string_free(ptr);
        }
    }

    /// <summary>
    /// Convert a null-terminated native string array to a managed string array and free the native memory.
    /// </summary>
    public static string[] PtrToStringArrayAndFree(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
            return Array.Empty<string>();

        try
        {
            var result = new List<string>();
            int offset = 0;
            while (true)
            {
                var strPtr = Marshal.ReadIntPtr(ptr, offset * IntPtr.Size);
                if (strPtr == IntPtr.Zero)
                    break;
                var str = Marshal.PtrToStringUTF8(strPtr);
                if (str != null)
                    result.Add(str);
                offset++;
            }
            return result.ToArray();
        }
        finally
        {
            pax_string_array_free(ptr);
        }
    }
}

/// <summary>
/// Result type for FFI operations.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
internal struct PaxResult
{
    [MarshalAs(UnmanagedType.I1)]
    public bool Success;
    public IntPtr ErrorMessage;

    public void ThrowIfError()
    {
        if (!Success && ErrorMessage != IntPtr.Zero)
        {
            var message = Marshal.PtrToStringUTF8(ErrorMessage) ?? "Unknown error";
            NativeMethods.pax_result_free(ref this);
            throw new PaxException(message);
        }
    }
}

/// <summary>
/// Value type enumeration matching the native enum.
/// </summary>
internal enum PaxValueType
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
