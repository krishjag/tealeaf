using System.Globalization;
using System.Reflection;
using System.Text;

namespace TeaLeaf;

/// <summary>
/// Runtime helper for TeaLeaf text formatting.
/// Provides the same string operations the source generator emits inline,
/// for use by the reflection-based TeaLeafSerializer.
/// </summary>
public static class TeaLeafTextHelper
{
    /// <summary>
    /// Converts a PascalCase name to snake_case.
    /// </summary>
    /// <param name="name">The PascalCase name to convert (e.g., "MyProperty").</param>
    /// <returns>The snake_case equivalent (e.g., "my_property").</returns>
    public static string ToSnakeCase(string name)
    {
        if (string.IsNullOrEmpty(name)) return name;

        var sb = new StringBuilder(name.Length + 4);
        for (int i = 0; i < name.Length; i++)
        {
            char c = name[i];
            if (char.IsUpper(c))
            {
                if (i > 0) sb.Append('_');
                sb.Append(char.ToLowerInvariant(c));
            }
            else
            {
                sb.Append(c);
            }
        }
        return sb.ToString();
    }

    /// <summary>
    /// Returns true if the string value needs quoting in TeaLeaf text format.
    /// </summary>
    /// <param name="value">The string value to check.</param>
    /// <returns>True if the value contains special characters, reserved words, or could be misinterpreted as a number.</returns>
    public static bool NeedsQuoting(string value)
    {
        if (string.IsNullOrEmpty(value)) return true;
        if (value == "true" || value == "false" || value == "~" || value == "null") return true;

        // Starts with +/- or a digit could be confused with number literals
        if (value[0] == '+' || value[0] == '-' || char.IsDigit(value[0]))
            return true;

        foreach (char c in value)
        {
            if (c == ' ' || c == ':' || c == '{' || c == '}' || c == '[' || c == ']' ||
                c == '"' || c == '\'' || c == '\\' || c == '\n' || c == '\r' || c == '\t' ||
                c == '#' || c == ',' || c == '@' || c == '(' || c == ')' || c == '/')
                return true;
        }

        // Check if it looks like a number
        if (double.TryParse(value, NumberStyles.Any, CultureInfo.InvariantCulture, out _))
            return true;

        return false;
    }

    /// <summary>
    /// Quotes a string value if needed for TeaLeaf text format.
    /// </summary>
    /// <param name="value">The string value to potentially quote.</param>
    /// <returns>The original value if safe, or a double-quoted and escaped version.</returns>
    public static string QuoteIfNeeded(string value)
    {
        if (!NeedsQuoting(value)) return value;
        return "\"" + EscapeString(value) + "\"";
    }

    /// <summary>
    /// Escapes special characters in a string for TeaLeaf text format.
    /// </summary>
    /// <param name="value">The string value to escape.</param>
    /// <returns>The escaped string with backslash, quote, newline, carriage return, and tab characters escaped.</returns>
    public static string EscapeString(string value)
    {
        if (string.IsNullOrEmpty(value)) return value;

        var sb = new StringBuilder(value.Length);
        foreach (char c in value)
        {
            switch (c)
            {
                case '\\': sb.Append("\\\\"); break;
                case '"': sb.Append("\\\""); break;
                case '\n': sb.Append("\\n"); break;
                case '\r': sb.Append("\\r"); break;
                case '\t': sb.Append("\\t"); break;
                default: sb.Append(c); break;
            }
        }
        return sb.ToString();
    }

    /// <summary>
    /// Formats a value as a TeaLeaf text string based on its type.
    /// </summary>
    /// <param name="sb">The StringBuilder to append the formatted value to.</param>
    /// <param name="value">The value to format, or null for the TeaLeaf null literal (~).</param>
    /// <param name="valueType">The declared type of the value, used to select the formatting strategy.</param>
    public static void AppendValue(StringBuilder sb, object? value, Type valueType)
    {
        if (value is null)
        {
            sb.Append('~');
            return;
        }

        var underlyingType = Nullable.GetUnderlyingType(valueType) ?? valueType;

        if (underlyingType == typeof(bool))
        {
            sb.Append((bool)value ? "true" : "false");
        }
        else if (underlyingType == typeof(int))
        {
            sb.Append(((int)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(long))
        {
            sb.Append(((long)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(double))
        {
            sb.Append(((double)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(float))
        {
            sb.Append(((float)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(string))
        {
            sb.Append(QuoteIfNeeded((string)value));
        }
        else if (underlyingType == typeof(uint))
        {
            sb.Append(((uint)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(ulong))
        {
            sb.Append(((ulong)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(short))
        {
            sb.Append(((short)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(byte))
        {
            sb.Append(((byte)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(decimal))
        {
            sb.Append(((decimal)value).ToString(CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(DateTime))
        {
            sb.Append(((DateTime)value).ToUniversalTime().ToString("o", CultureInfo.InvariantCulture));
        }
        else if (underlyingType == typeof(DateTimeOffset))
        {
            sb.Append(((DateTimeOffset)value).ToUniversalTime().ToString("o", CultureInfo.InvariantCulture));
        }
        else if (underlyingType.IsEnum)
        {
            sb.Append(ToSnakeCase(value.ToString()!));
        }
        else
        {
            // Fallback: quote the ToString()
            sb.Append(QuoteIfNeeded(value.ToString() ?? ""));
        }
    }

    /// <summary>
    /// Maps a C# type to its TeaLeaf schema type name.
    /// </summary>
    /// <param name="type">The C# type to map (e.g., typeof(int), typeof(string)).</param>
    /// <returns>The TeaLeaf type name (e.g., "int", "string", "timestamp", "[]int").</returns>
    public static string GetTLTypeName(Type type)
    {
        var underlying = Nullable.GetUnderlyingType(type) ?? type;

        if (underlying == typeof(bool)) return "bool";
        if (underlying == typeof(int)) return "int";
        if (underlying == typeof(long)) return "int64";
        if (underlying == typeof(short)) return "int16";
        if (underlying == typeof(sbyte)) return "int8";
        if (underlying == typeof(uint)) return "uint";
        if (underlying == typeof(ulong)) return "uint64";
        if (underlying == typeof(ushort)) return "uint16";
        if (underlying == typeof(byte)) return "uint8";
        if (underlying == typeof(double)) return "float";
        if (underlying == typeof(float)) return "float32";
        if (underlying == typeof(decimal)) return "float";
        if (underlying == typeof(string)) return "string";
        if (underlying == typeof(DateTime)) return "timestamp";
        if (underlying == typeof(DateTimeOffset)) return "timestamp";
        if (underlying == typeof(byte[])) return "bytes";
        if (underlying.IsEnum) return "string";

        // Check for List<T>
        if (underlying.IsGenericType && underlying.GetGenericTypeDefinition() == typeof(List<>))
        {
            var elemType = underlying.GetGenericArguments()[0];
            return "[]" + GetTLTypeName(elemType);
        }

        // Check for Dictionary<string, T>
        if (underlying.IsGenericType && underlying.GetGenericTypeDefinition() == typeof(Dictionary<,>))
        {
            return "object";
        }

        // Nested TeaLeaf object — return struct name for schema reference
        var teaLeafAttr = underlying.GetCustomAttribute<Annotations.TeaLeafAttribute>();
        if (teaLeafAttr != null)
        {
            return teaLeafAttr.StructName ?? ToSnakeCase(underlying.Name);
        }

        // Unknown type: generic object
        return "object";
    }
}
