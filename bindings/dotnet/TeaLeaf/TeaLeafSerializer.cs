using System.Collections;
using System.Globalization;
using System.Reflection;
using System.Text;
using TeaLeaf.Internal;

namespace TeaLeaf;

/// <summary>
/// Reflection-based TeaLeaf serializer for types annotated with [TeaLeaf].
/// Handles generic types and any types that cannot be source-generated.
/// For concrete (non-generic) types, prefer the source-generated methods
/// (ToTeaLeafText, FromTeaLeaf, etc.) for better performance.
/// </summary>
public static class TeaLeafSerializer
{
    // ----------------------------------------------------------------
    // Serialization
    // ----------------------------------------------------------------

    /// <summary>
    /// Serializes the object body (fields only, no schema or key wrapper) to TeaLeaf text.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="value">The object to serialize.</param>
    /// <returns>A string containing the serialized fields in TeaLeaf text format.</returns>
    /// <exception cref="ArgumentNullException">Thrown if <paramref name="value"/> is null.</exception>
    public static string ToText<T>(T value) where T : class
    {
        ArgumentNullException.ThrowIfNull(value);
        var info = TeaLeafTypeInfo.GetOrCreate(value.GetType());
        var sb = new StringBuilder(256);
        WriteObjectBody(sb, value, info, "    ");
        return sb.ToString();
    }

    /// <summary>
    /// Serializes the object to a full TeaLeaf document string (schema + data).
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="value">The object to serialize.</param>
    /// <param name="key">Optional top-level key name. Defaults to the type's struct name.</param>
    /// <returns>A complete TeaLeaf document string including @struct definitions and data.</returns>
    /// <exception cref="ArgumentNullException">Thrown if <paramref name="value"/> is null.</exception>
    public static string ToDocument<T>(T value, string? key = null) where T : class
    {
        ArgumentNullException.ThrowIfNull(value);
        var info = TeaLeafTypeInfo.GetOrCreate(value.GetType());
        var docKey = key ?? info.Key;

        var sb = new StringBuilder(512);

        // Schema (recursive — emits all nested @struct declarations)
        var emitted = new HashSet<string>();
        WriteAllSchemas(sb, info, emitted);
        sb.AppendLine();

        // Data
        sb.Append(docKey);
        sb.AppendLine(": {");
        WriteObjectBody(sb, value, info, "    ");
        sb.AppendLine("}");

        return sb.ToString();
    }

    /// <summary>
    /// Serializes the object to a TLDocument (parsed native document).
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="value">The object to serialize.</param>
    /// <param name="key">Optional top-level key name. Defaults to the type's struct name.</param>
    /// <returns>A <see cref="TLDocument"/> ready for binary compilation or JSON export. The caller must dispose.</returns>
    public static TLDocument ToTLDocument<T>(T value, string? key = null) where T : class
    {
        var text = ToDocument(value, key);
        return TLDocument.Parse(text);
    }

    /// <summary>
    /// Serializes the object to JSON via TLDocument.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="value">The object to serialize.</param>
    /// <param name="key">Optional top-level key name. Defaults to the type's struct name.</param>
    /// <returns>A JSON string representation of the object.</returns>
    public static string ToJson<T>(T value, string? key = null) where T : class
    {
        using var doc = ToTLDocument(value, key);
        return doc.ToJson();
    }

    /// <summary>
    /// Compiles the object to binary TeaLeaf format.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="value">The object to serialize.</param>
    /// <param name="path">The output file path for the .tlbx binary.</param>
    /// <param name="key">Optional top-level key name. Defaults to the type's struct name.</param>
    /// <param name="compress">Whether to apply compression to the binary output.</param>
    public static void Compile<T>(T value, string path, string? key = null, bool compress = false) where T : class
    {
        using var doc = ToTLDocument(value, key);
        doc.Compile(path, compress);
    }

    // ----------------------------------------------------------------
    // Deserialization
    // ----------------------------------------------------------------

    /// <summary>
    /// Deserializes an object from a TLDocument.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.
    /// May have a parameterless constructor or a parameterized constructor whose parameter names
    /// match property names (case-insensitive).</typeparam>
    /// <param name="doc">The TLDocument to read from.</param>
    /// <param name="key">Optional top-level key to look up. Defaults to the type's struct name.</param>
    /// <returns>A deserialized instance of <typeparamref name="T"/>.</returns>
    /// <exception cref="TLException">Thrown if the key is not found in the document.</exception>
    public static T FromDocument<T>(TLDocument doc, string? key = null) where T : class
    {
        var info = TeaLeafTypeInfo.GetOrCreate(typeof(T));
        var docKey = key ?? info.Key;

        using var value = doc[docKey];
        if (value == null)
            throw new TLException($"Key '{docKey}' not found in document");

        return FromValue<T>(value);
    }

    /// <summary>
    /// Deserializes an object from a TLValue (object type).
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.
    /// May have a parameterless constructor or a parameterized constructor whose parameter names
    /// match property names (case-insensitive).</typeparam>
    /// <param name="value">A TLValue of type Object containing the serialized fields.</param>
    /// <returns>A deserialized instance of <typeparamref name="T"/>.</returns>
    public static T FromValue<T>(TLValue value) where T : class
    {
        return (T)FromValueInternal(value, typeof(T));
    }

    /// <summary>
    /// Deserializes an object from TeaLeaf text.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.
    /// May have a parameterless constructor or a parameterized constructor whose parameter names
    /// match property names (case-insensitive).</typeparam>
    /// <param name="tlText">The TeaLeaf text to parse and deserialize.</param>
    /// <param name="key">Optional top-level key to look up. Defaults to the type's struct name.</param>
    /// <returns>A deserialized instance of <typeparamref name="T"/>.</returns>
    /// <exception cref="TLException">Thrown if the key is not found or the text cannot be parsed.</exception>
    public static T FromText<T>(string tlText, string? key = null) where T : class
    {
        using var doc = TLDocument.Parse(tlText);
        return FromDocument<T>(doc, key);
    }

    // ----------------------------------------------------------------
    // Schema
    // ----------------------------------------------------------------

    /// <summary>
    /// Gets the TeaLeaf schema definition for a type.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <returns>An @struct definition string for the type.</returns>
    public static string GetSchema<T>() where T : class
    {
        var info = TeaLeafTypeInfo.GetOrCreate(typeof(T));
        var sb = new StringBuilder(128);
        WriteSchema(sb, info);
        return sb.ToString();
    }

    // ----------------------------------------------------------------
    // Collections
    // ----------------------------------------------------------------

    /// <summary>
    /// Serializes a collection to a TeaLeaf document string.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="items">The collection of objects to serialize.</param>
    /// <param name="key">The top-level key name for the array in the document.</param>
    /// <returns>A complete TeaLeaf document string with schema and table data.</returns>
    /// <exception cref="ArgumentNullException">Thrown if <paramref name="items"/> is null.</exception>
    public static string ToText<T>(IEnumerable<T> items, string key) where T : class
    {
        ArgumentNullException.ThrowIfNull(items);
        var info = TeaLeafTypeInfo.GetOrCreate(typeof(T));

        var sb = new StringBuilder(512);

        // Schema (recursive — emits all nested @struct declarations)
        var emitted = new HashSet<string>();
        WriteAllSchemas(sb, info, emitted);
        sb.AppendLine();

        // Table data
        sb.Append(key);
        sb.Append(": @table ");
        sb.Append(info.StructName);
        sb.Append(" [");

        var itemList = items.ToList();
        if (itemList.Count == 0)
        {
            sb.AppendLine("]");
        }
        else
        {
            sb.AppendLine();
            bool first = true;
            foreach (var item in itemList)
            {
                if (!first) sb.AppendLine(",");
                first = false;
                sb.Append("    ");
                WriteTupleValue(sb, item!, info);
            }
            sb.AppendLine();
            sb.AppendLine("]");
        }

        return sb.ToString();
    }

    /// <summary>
    /// Serializes a collection to a TLDocument.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="items">The collection of objects to serialize.</param>
    /// <param name="key">The top-level key name for the array in the document.</param>
    /// <returns>A <see cref="TLDocument"/> containing the serialized collection. The caller must dispose.</returns>
    public static TLDocument ToTLDocument<T>(IEnumerable<T> items, string key) where T : class
    {
        var text = ToText(items, key);
        return TLDocument.Parse(text);
    }

    /// <summary>
    /// Deserializes a list from a TLDocument.
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="doc">The TLDocument to read from.</param>
    /// <param name="key">The top-level key for the array in the document.</param>
    /// <returns>A list of deserialized <typeparamref name="T"/> instances.</returns>
    /// <exception cref="TLException">Thrown if the key is not found in the document.</exception>
    public static List<T> FromList<T>(TLDocument doc, string key) where T : class
    {
        using var value = doc[key];
        if (value == null)
            throw new TLException($"Key '{key}' not found in document");
        return FromList<T>(value);
    }

    /// <summary>
    /// Deserializes a list from a TLValue (array type).
    /// </summary>
    /// <typeparam name="T">A class annotated with <see cref="Annotations.TeaLeafAttribute"/>.</typeparam>
    /// <param name="arrayValue">A TLValue of type Array containing the serialized elements.</param>
    /// <returns>A list of deserialized <typeparamref name="T"/> instances.</returns>
    /// <exception cref="TLException">Thrown if the value is not of type Array.</exception>
    public static List<T> FromList<T>(TLValue arrayValue) where T : class
    {
        if (arrayValue.Type != TLType.Array)
            throw new TLException($"Expected array value but got {arrayValue.Type}");

        var result = new List<T>();
        foreach (var elem in arrayValue.AsArray())
        {
            using (elem)
            {
                result.Add(FromValue<T>(elem));
            }
        }
        return result;
    }

    // ----------------------------------------------------------------
    // Internal helpers
    // ----------------------------------------------------------------

    private static void WriteAllSchemas(StringBuilder sb, TeaLeafTypeInfo info, HashSet<string> emitted)
    {
        // Recurse into nested types first (dependency order: leaf types emitted first)
        foreach (var prop in info.Properties)
        {
            TeaLeafTypeInfo? nestedInfo = null;

            if (prop.IsNestedTeaLeaf)
            {
                var nestedType = Nullable.GetUnderlyingType(prop.PropertyType) ?? prop.PropertyType;
                nestedInfo = TeaLeafTypeInfo.GetOrCreate(nestedType);
            }
            else if (prop.IsList && prop.ElementType != null &&
                     prop.ElementType.GetCustomAttribute<Annotations.TeaLeafAttribute>() != null)
            {
                nestedInfo = TeaLeafTypeInfo.GetOrCreate(prop.ElementType);
            }

            if (nestedInfo != null && !emitted.Contains(nestedInfo.StructName))
            {
                WriteAllSchemas(sb, nestedInfo, emitted);
            }
        }

        // Emit this type's schema (if not already emitted)
        if (emitted.Add(info.StructName))
        {
            WriteSchema(sb, info);
        }
    }

    private static void WriteSchema(StringBuilder sb, TeaLeafTypeInfo info)
    {
        sb.Append("@struct ");
        sb.Append(info.StructName);
        sb.Append(" (");

        bool first = true;
        foreach (var prop in info.Properties)
        {
            if (!first) sb.Append(", ");
            first = false;
            sb.Append(prop.TLName);
            sb.Append(": ");
            sb.Append(prop.TLType);
        }

        sb.AppendLine(")");
    }

    private static void WriteObjectBody(StringBuilder sb, object obj, TeaLeafTypeInfo info, string indent)
    {
        foreach (var prop in info.Properties)
        {
            var value = prop.Getter(obj);

            if (value == null && prop.IsNullable)
            {
                sb.Append(indent);
                sb.Append(prop.TLName);
                sb.AppendLine(": ~");
                continue;
            }

            if (value == null) continue;

            if (prop.IsList)
            {
                WriteList(sb, (IList)value, prop, indent);
            }
            else if (prop.IsDictionary)
            {
                WriteDictionary(sb, (IDictionary)value, prop, indent);
            }
            else if (prop.IsNestedTeaLeaf)
            {
                WriteNestedObject(sb, value, prop, indent);
            }
            else
            {
                sb.Append(indent);
                sb.Append(prop.TLName);
                sb.Append(": ");
                TeaLeafTextHelper.AppendValue(sb, value, prop.PropertyType);
                sb.AppendLine();
            }
        }
    }

    private static void WriteList(StringBuilder sb, IList list, TeaLeafPropertyInfo prop, string indent)
    {
        sb.Append(indent);
        sb.Append(prop.TLName);
        sb.Append(": ");

        var elemType = prop.ElementType ?? typeof(object);
        bool isNestedTeaLeaf = elemType.GetCustomAttribute<Annotations.TeaLeafAttribute>() != null;

        if (isNestedTeaLeaf)
        {
            var nestedInfo = TeaLeafTypeInfo.GetOrCreate(elemType);
            sb.Append("@table ");
            sb.Append(nestedInfo.StructName);
            sb.Append(" [");

            if (list.Count == 0)
            {
                sb.AppendLine("]");
                return;
            }

            sb.AppendLine();
            var tupleIndent = indent + "    ";
            bool firstTuple = true;
            foreach (var item in list)
            {
                if (item == null) continue;
                if (!firstTuple) sb.AppendLine(",");
                firstTuple = false;
                sb.Append(tupleIndent);
                var itemInfo = TeaLeafTypeInfo.GetOrCreate(item.GetType());
                WriteTupleValue(sb, item, itemInfo);
            }
            sb.AppendLine();
            sb.Append(indent);
            sb.AppendLine("]");
        }
        else
        {
            sb.Append('[');
            for (int i = 0; i < list.Count; i++)
            {
                if (i > 0) sb.Append(", ");
                TeaLeafTextHelper.AppendValue(sb, list[i], elemType);
            }
            sb.AppendLine("]");
        }
    }

    private static void WriteTupleValue(StringBuilder sb, object obj, TeaLeafTypeInfo info)
    {
        sb.Append('(');
        bool first = true;
        foreach (var prop in info.Properties)
        {
            if (!first) sb.Append(", ");
            first = false;

            var value = prop.Getter(obj);
            if (value == null)
            {
                sb.Append('~');
                continue;
            }

            if (prop.IsList)
            {
                WriteTupleList(sb, (IList)value, prop);
            }
            else if (prop.IsDictionary)
            {
                WriteTupleDictionary(sb, (IDictionary)value);
            }
            else if (prop.IsNestedTeaLeaf)
            {
                var nestedInfo = TeaLeafTypeInfo.GetOrCreate(value.GetType());
                WriteTupleValue(sb, value, nestedInfo);
            }
            else if (prop.IsEnum)
            {
                sb.Append(TeaLeafTextHelper.ToSnakeCase(value.ToString()!));
            }
            else
            {
                TeaLeafTextHelper.AppendValue(sb, value, prop.PropertyType);
            }
        }
        sb.Append(')');
    }

    private static void WriteTupleList(StringBuilder sb, IList list, TeaLeafPropertyInfo prop)
    {
        sb.Append('[');
        var elemType = prop.ElementType ?? typeof(object);
        bool isNestedTeaLeaf = elemType.GetCustomAttribute<Annotations.TeaLeafAttribute>() != null;

        for (int i = 0; i < list.Count; i++)
        {
            if (i > 0) sb.Append(", ");
            var item = list[i];
            if (item == null)
            {
                sb.Append('~');
            }
            else if (isNestedTeaLeaf)
            {
                var nestedInfo = TeaLeafTypeInfo.GetOrCreate(item.GetType());
                WriteTupleValue(sb, item, nestedInfo);
            }
            else
            {
                TeaLeafTextHelper.AppendValue(sb, item, elemType);
            }
        }
        sb.Append(']');
    }

    private static void WriteTupleDictionary(StringBuilder sb, IDictionary dict)
    {
        sb.Append('{');
        bool first = true;
        foreach (DictionaryEntry entry in dict)
        {
            if (!first) sb.Append(", ");
            first = false;
            sb.Append(TeaLeafTextHelper.QuoteIfNeeded(entry.Key?.ToString() ?? ""));
            sb.Append(": ");
            if (entry.Value != null)
                TeaLeafTextHelper.AppendValue(sb, entry.Value, entry.Value.GetType());
            else
                sb.Append('~');
        }
        sb.Append('}');
    }

    private static void WriteDictionary(StringBuilder sb, IDictionary dict, TeaLeafPropertyInfo prop, string indent)
    {
        sb.Append(indent);
        sb.Append(prop.TLName);
        sb.AppendLine(": {");

        var innerIndent = indent + "    ";
        foreach (DictionaryEntry entry in dict)
        {
            sb.Append(innerIndent);
            sb.Append(TeaLeafTextHelper.QuoteIfNeeded(entry.Key?.ToString() ?? ""));
            sb.Append(": ");
            if (entry.Value != null)
                TeaLeafTextHelper.AppendValue(sb, entry.Value, entry.Value.GetType());
            else
                sb.Append('~');
            sb.AppendLine();
        }

        sb.Append(indent);
        sb.AppendLine("}");
    }

    private static void WriteNestedObject(StringBuilder sb, object value, TeaLeafPropertyInfo prop, string indent)
    {
        var nestedInfo = TeaLeafTypeInfo.GetOrCreate(value.GetType());
        sb.Append(indent);
        sb.Append(prop.TLName);
        sb.AppendLine(": {");
        WriteObjectBody(sb, value, nestedInfo, indent + "    ");
        sb.Append(indent);
        sb.AppendLine("}");
    }

    private static object FromValueInternal(TLValue value, Type targetType)
    {
        var info = TeaLeafTypeInfo.GetOrCreate(targetType);

        if (info.ParameterizedConstructor != null)
            return FromValueWithConstructor(value, info);

        // When no parameterless constructor AND no usable parameterized constructor,
        // create instance without calling any constructor and set properties via setters.
        var result = info.UseUninitializedObject
            ? System.Runtime.CompilerServices.RuntimeHelpers.GetUninitializedObject(targetType)
            : Activator.CreateInstance(targetType)!;

        foreach (var prop in info.Properties)
        {
            using var field = value.GetField(prop.TLName);
            if (field == null || field.IsNull)
                continue;

            var propValue = ReadProperty(field, prop);
            if (propValue != null)
                prop.Setter?.Invoke(result, propValue);
        }

        return result;
    }

    private static object FromValueWithConstructor(TLValue value, TeaLeafTypeInfo info)
    {
        var ctorMappings = info.ConstructorParamMappings!;
        var ctor = info.ParameterizedConstructor!;

        // Read all fields into a dictionary for lookup
        var fieldValues = new Dictionary<string, object?>();
        foreach (var prop in info.Properties)
        {
            using var field = value.GetField(prop.TLName);
            if (field == null || field.IsNull)
                continue;

            var propValue = ReadProperty(field, prop);
            fieldValues[prop.TLName] = propValue;
        }

        // Build constructor arguments
        var ctorArgs = new object?[ctorMappings.Length];
        var ctorParamTLNames = new HashSet<string>(StringComparer.Ordinal);

        for (int i = 0; i < ctorMappings.Length; i++)
        {
            var mapping = ctorMappings[i];
            ctorParamTLNames.Add(mapping.TLName);

            if (fieldValues.TryGetValue(mapping.TLName, out var val))
            {
                ctorArgs[i] = val;
            }
            else if (mapping.HasDefaultValue)
            {
                ctorArgs[i] = mapping.DefaultValue;
            }
            else
            {
                ctorArgs[i] = mapping.ParameterType.IsValueType
                    ? Activator.CreateInstance(mapping.ParameterType)
                    : null;
            }
        }

        // Create instance via constructor
        var result = ctor.Invoke(ctorArgs);

        // Set remaining properties that were NOT constructor parameters
        foreach (var prop in info.Properties)
        {
            if (ctorParamTLNames.Contains(prop.TLName)) continue;
            if (prop.Setter == null) continue;

            if (fieldValues.TryGetValue(prop.TLName, out var val) && val != null)
            {
                prop.Setter(result, val);
            }
        }

        return result;
    }

    private static object? ReadProperty(TLValue field, TeaLeafPropertyInfo prop)
    {
        if (prop.IsList)
            return ReadList(field, prop);

        if (prop.IsDictionary)
            return ReadDictionary(field, prop);

        if (prop.IsNestedTeaLeaf)
            return FromValueInternal(field, Nullable.GetUnderlyingType(prop.PropertyType) ?? prop.PropertyType);

        return ReadPrimitive(field, prop);
    }

    private static object? ReadPrimitive(TLValue field, TeaLeafPropertyInfo prop)
    {
        var underlying = Nullable.GetUnderlyingType(prop.PropertyType) ?? prop.PropertyType;

        if (underlying == typeof(bool))
            return field.AsBool();

        if (underlying == typeof(int))
        {
            var v = field.AsInt();
            return v.HasValue ? (int)v.Value : null;
        }

        if (underlying == typeof(long))
            return field.AsInt();

        if (underlying == typeof(short))
        {
            var v = field.AsInt();
            return v.HasValue ? (short)v.Value : null;
        }

        if (underlying == typeof(byte))
        {
            var v = field.AsInt();
            return v.HasValue ? (byte)v.Value : null;
        }

        if (underlying == typeof(uint))
        {
            var v = field.AsUInt();
            return v.HasValue ? (uint)v.Value : null;
        }

        if (underlying == typeof(ulong))
            return field.AsUInt();

        if (underlying == typeof(double))
        {
            var v = field.AsFloat();
            if (v.HasValue) return v.Value;
            // Whole-number values are parsed as int; coerce to double
            var i = field.AsInt();
            if (i.HasValue) return (double)i.Value;
            return null;
        }

        if (underlying == typeof(float))
        {
            var v = field.AsFloat();
            if (v.HasValue) return (float)v.Value;
            var i = field.AsInt();
            if (i.HasValue) return (float)i.Value;
            return null;
        }

        if (underlying == typeof(decimal))
        {
            var v = field.AsFloat();
            if (v.HasValue) return (decimal)v.Value;
            var i = field.AsInt();
            if (i.HasValue) return (decimal)i.Value;
        }

        if (underlying == typeof(string))
            return field.AsString();

        if (underlying == typeof(DateTime))
        {
            var dt = field.AsDateTime();
            return dt?.DateTime;
        }

        if (underlying == typeof(DateTimeOffset))
            return field.AsDateTime();

        if (underlying == typeof(byte[]))
            return field.AsBytes();

        if (underlying.IsEnum)
        {
            var str = field.AsString();
            if (str == null) return null;
            // Convert snake_case back to PascalCase enum value
            return ParseEnumFromSnakeCase(underlying, str);
        }

        return null;
    }

    private static object? ParseEnumFromSnakeCase(Type enumType, string snakeCaseValue)
    {
        // Try direct parse first (handles PascalCase values)
        if (Enum.TryParse(enumType, snakeCaseValue, ignoreCase: true, out var result))
            return result;

        // Try matching by converting each enum value to snake_case
        foreach (var name in Enum.GetNames(enumType))
        {
            if (TeaLeafTextHelper.ToSnakeCase(name) == snakeCaseValue)
                return Enum.Parse(enumType, name);
        }

        throw new TLException($"Cannot parse '{snakeCaseValue}' as {enumType.Name}");
    }

    private static object? ReadList(TLValue field, TeaLeafPropertyInfo prop)
    {
        if (field.Type != TLType.Array) return null;

        var elemType = prop.ElementType ?? typeof(object);
        var listType = typeof(List<>).MakeGenericType(elemType);
        var list = (IList)Activator.CreateInstance(listType)!;

        bool isNestedTeaLeaf = elemType.GetCustomAttribute<Annotations.TeaLeafAttribute>() != null;

        foreach (var elem in field.AsArray())
        {
            using (elem)
            {
                if (isNestedTeaLeaf)
                {
                    list.Add(FromValueInternal(elem, elemType));
                }
                else
                {
                    list.Add(ReadPrimitiveValue(elem, elemType));
                }
            }
        }

        return list;
    }

    private static object? ReadDictionary(TLValue field, TeaLeafPropertyInfo prop)
    {
        if (field.Type != TLType.Object) return null;

        var valueType = prop.ElementType ?? typeof(object);
        var dictType = typeof(Dictionary<,>).MakeGenericType(typeof(string), valueType);
        var dict = (IDictionary)Activator.CreateInstance(dictType)!;

        foreach (var key in field.GetObjectKeys())
        {
            using var val = field.GetField(key);
            if (val != null)
            {
                dict[key] = ReadPrimitiveValue(val, valueType);
            }
        }

        return dict;
    }

    private static object? ReadPrimitiveValue(TLValue value, Type targetType)
    {
        var underlying = Nullable.GetUnderlyingType(targetType) ?? targetType;

        if (value.IsNull) return null;

        if (underlying == typeof(bool)) return value.AsBool();
        if (underlying == typeof(int)) { var v = value.AsInt(); return v.HasValue ? (int)v.Value : null; }
        if (underlying == typeof(long)) return value.AsInt();
        if (underlying == typeof(uint)) { var v = value.AsUInt(); return v.HasValue ? (uint)v.Value : null; }
        if (underlying == typeof(ulong)) return value.AsUInt();
        if (underlying == typeof(double)) { var f = value.AsFloat(); if (f.HasValue) return f.Value; var i = value.AsInt(); return i.HasValue ? (double)i.Value : null; }
        if (underlying == typeof(float)) { var f = value.AsFloat(); if (f.HasValue) return (float)f.Value; var i = value.AsInt(); return i.HasValue ? (float)i.Value : null; }
        if (underlying == typeof(string)) return value.AsString();
        if (underlying == typeof(byte[])) return value.AsBytes();
        if (underlying.IsEnum)
        {
            var str = value.AsString();
            return str != null ? ParseEnumFromSnakeCase(underlying, str) : null;
        }

        // Nested TeaLeaf object
        if (underlying.GetCustomAttribute<Annotations.TeaLeafAttribute>() != null)
            return FromValueInternal(value, underlying);

        return value.ToObject();
    }
}
