using System.Text;

namespace TeaLeaf.Generators;

internal static class TLTextEmitter
{
    public static void EmitSerializationMethods(StringBuilder sb, TeaLeafModel model)
    {
        string key = model.DefaultKey;

        // GetTeaLeafSchema()
        EmitGetSchema(sb, model);
        sb.AppendLine();

        // ToTeaLeafText()
        EmitToTeaLeafText(sb, model);
        sb.AppendLine();

        // WriteTeaLeafObjectBody()
        EmitWriteObjectBody(sb, model);
        sb.AppendLine();

        // ToTeaLeafDocument()
        sb.AppendLine($"    /// <summary>Serializes to a complete .tl document string with schema.</summary>");
        sb.AppendLine($"    /// <param name=\"key\">The top-level key name. Defaults to the struct name.</param>");
        sb.AppendLine($"    /// <returns>A TeaLeaf text document including @struct definitions and data.</returns>");
        sb.AppendLine($"    public string ToTeaLeafDocument(string key = \"{key}\")");
        sb.AppendLine("    {");
        sb.AppendLine("        var sb = new System.Text.StringBuilder(512);");
        // GetTeaLeafSchema() now recursively includes all nested type schemas
        sb.AppendLine("        sb.AppendLine(GetTeaLeafSchema());");
        sb.AppendLine("        sb.AppendLine();");
        sb.AppendLine("        sb.Append(key);");
        sb.AppendLine("        sb.AppendLine(\": {\");");
        sb.AppendLine("        WriteTeaLeafObjectBody(sb, \"    \");");
        sb.AppendLine("        sb.AppendLine(\"}\");");
        sb.AppendLine("        return sb.ToString();");
        sb.AppendLine("    }");
        sb.AppendLine();

        // ToTLDocument()
        sb.AppendLine($"    /// <summary>Serializes to an in-memory TLDocument via the native engine.</summary>");
        sb.AppendLine($"    /// <param name=\"key\">The top-level key name. Defaults to the struct name.</param>");
        sb.AppendLine($"    /// <returns>A parsed <see cref=\"TeaLeaf.TLDocument\"/> ready for binary compilation or JSON export.</returns>");
        sb.AppendLine($"    public TeaLeaf.TLDocument ToTLDocument(string key = \"{key}\")");
        sb.AppendLine("    {");
        sb.AppendLine("        return TeaLeaf.TLDocument.Parse(ToTeaLeafDocument(key));");
        sb.AppendLine("    }");
        sb.AppendLine();

        // ToTeaLeafJson()
        sb.AppendLine($"    /// <summary>Serializes to JSON via TeaLeaf's native JSON emitter.</summary>");
        sb.AppendLine($"    /// <param name=\"key\">The top-level key name. Defaults to the struct name.</param>");
        sb.AppendLine($"    /// <returns>A JSON string representation of this object.</returns>");
        sb.AppendLine($"    public string ToTeaLeafJson(string key = \"{key}\")");
        sb.AppendLine("    {");
        sb.AppendLine("        using var doc = ToTLDocument(key);");
        sb.AppendLine("        return doc.ToJson();");
        sb.AppendLine("    }");
        sb.AppendLine();

        // CompileToTeaLeaf()
        sb.AppendLine($"    /// <summary>Compiles to a binary .tlbx file.</summary>");
        sb.AppendLine($"    /// <param name=\"path\">The output file path for the .tlbx binary.</param>");
        sb.AppendLine($"    /// <param name=\"key\">The top-level key name. Defaults to the struct name.</param>");
        sb.AppendLine($"    /// <param name=\"compress\">Whether to apply compression to the binary output.</param>");
        sb.AppendLine($"    public void CompileToTeaLeaf(string path, string key = \"{key}\", bool compress = false)");
        sb.AppendLine("    {");
        sb.AppendLine("        using var doc = ToTLDocument(key);");
        sb.AppendLine("        doc.Compile(path, compress);");
        sb.AppendLine("    }");
    }

    private static void EmitGetSchema(StringBuilder sb, TeaLeafModel model)
    {
        var fields = new List<string>();
        foreach (var prop in model.Properties)
        {
            if (prop.IsSkipped) continue;
            string typeStr = prop.TLType;
            if (prop.IsNullable && !typeStr.EndsWith("?"))
                typeStr += "?";
            fields.Add($"{prop.TLName}: {typeStr}");
        }

        string fieldList = string.Join(", ", fields);

        // Collect unique nested C# type names that have [TeaLeaf] and thus CollectTeaLeafSchemas()
        var nestedCSharpTypes = new HashSet<string>();
        foreach (var prop in model.Properties)
        {
            if (prop.IsSkipped) continue;
            string? nestedTypeName = null;
            if (prop.Kind == PropertyKind.NestedObject)
                nestedTypeName = prop.CSharpType.TrimEnd('?');
            else if (prop.Kind == PropertyKind.List && prop.IsNestedTeaLeafType && prop.CollectionElementType != null)
                nestedTypeName = prop.CollectionElementType;

            if (nestedTypeName != null)
                nestedCSharpTypes.Add(nestedTypeName);
        }

        // Public backward-compatible method: delegates to CollectTeaLeafSchemas()
        sb.AppendLine("    /// <summary>Generates a TeaLeaf @struct definition for this type and all nested types.</summary>");
        sb.AppendLine("    /// <returns>A string containing @struct declarations for this type and its dependencies.</returns>");
        sb.AppendLine("    public static string GetTeaLeafSchema()");
        sb.AppendLine("    {");
        sb.AppendLine("        var emitted = new System.Collections.Generic.HashSet<string>();");
        sb.AppendLine("        var sb = new System.Text.StringBuilder(256);");
        sb.AppendLine("        CollectTeaLeafSchemas(sb, emitted);");
        sb.AppendLine("        return sb.ToString();");
        sb.AppendLine("    }");
        sb.AppendLine();

        // Public dedup-aware recursive collector (public for cross-assembly access)
        sb.AppendLine("    /// <summary>Collects @struct definitions for this type and nested types, deduplicating by name.</summary>");
        sb.AppendLine("    public static void CollectTeaLeafSchemas(System.Text.StringBuilder sb, System.Collections.Generic.HashSet<string> emitted)");
        sb.AppendLine("    {");

        // Recursively collect from nested types first (dependency order)
        foreach (var nestedType in nestedCSharpTypes)
        {
            if (model.CrossAssemblyFallbackTypes.Contains(nestedType))
            {
                // Fallback for referenced assemblies compiled with older generator (no CollectTeaLeafSchemas)
                sb.AppendLine("        {");
                sb.AppendLine($"            var __schema = global::{nestedType}.GetTeaLeafSchema();");
                sb.AppendLine("            if (__schema.Length > 0) { if (sb.Length > 0) sb.AppendLine(); sb.Append(__schema); }");
                sb.AppendLine("        }");
            }
            else
            {
                sb.AppendLine($"        global::{nestedType}.CollectTeaLeafSchemas(sb, emitted);");
            }
        }

        // Emit own schema if not already emitted
        sb.AppendLine($"        if (emitted.Add(\"{model.StructName}\"))");
        sb.AppendLine("        {");
        sb.AppendLine("            if (sb.Length > 0) sb.AppendLine();");
        sb.AppendLine($"            sb.Append(\"@struct {model.StructName} ({fieldList})\");");
        sb.AppendLine("        }");

        sb.AppendLine("    }");
    }

    private static void EmitToTeaLeafText(StringBuilder sb, TeaLeafModel model)
    {
        sb.AppendLine("    /// <summary>Serializes this DTO's fields to TeaLeaf text format.</summary>");
        sb.AppendLine("    /// <returns>A string containing the serialized fields without schema or key wrapper.</returns>");
        sb.AppendLine("    public string ToTeaLeafText()");
        sb.AppendLine("    {");
        sb.AppendLine("        var sb = new System.Text.StringBuilder(256);");
        sb.AppendLine("        WriteTeaLeafObjectBody(sb, \"\");");
        sb.AppendLine("        return sb.ToString();");
        sb.AppendLine("    }");
    }

    private static void EmitWriteObjectBody(StringBuilder sb, TeaLeafModel model)
    {
        sb.AppendLine("    /// <summary>Writes fields to a StringBuilder with the given indentation.</summary>");
        sb.AppendLine("    /// <param name=\"sb\">The StringBuilder to append field text to.</param>");
        sb.AppendLine("    /// <param name=\"indent\">The indentation prefix for each line.</param>");
        sb.AppendLine("    public void WriteTeaLeafObjectBody(System.Text.StringBuilder sb, string indent)");
        sb.AppendLine("    {");

        foreach (var prop in model.Properties)
        {
            if (prop.IsSkipped) continue;
            EmitPropertyWrite(sb, prop);
        }

        sb.AppendLine("    }");
    }

    private static void EmitPropertyWrite(StringBuilder sb, TeaLeafProperty prop)
    {
        string name = prop.TLName;
        string access = prop.CSharpName;

        if (prop.IsCSharpNullable && prop.Kind == PropertyKind.Primitive)
        {
            // Nullable value type (int?, bool?, double?, etc.)
            sb.AppendLine($"        if ({access}.HasValue)");
            sb.AppendLine("        {");
            EmitNullablePrimitiveWrite(sb, prop, "            ");
            sb.AppendLine("        }");
            sb.AppendLine("        else");
            sb.AppendLine("        {");
            sb.AppendLine($"            sb.Append(indent);");
            sb.AppendLine($"            sb.AppendLine(\"{name}: ~\");");
            sb.AppendLine("        }");
        }
        else if (prop.IsCSharpNullable)
        {
            // Nullable reference type (string?, NestedObj?, etc.)
            sb.AppendLine($"        if ({access} is not null)");
            sb.AppendLine("        {");
            EmitPropertyValueWrite(sb, prop, "            ");
            sb.AppendLine("        }");
            sb.AppendLine("        else");
            sb.AppendLine("        {");
            sb.AppendLine($"            sb.Append(indent);");
            sb.AppendLine($"            sb.AppendLine(\"{name}: ~\");");
            sb.AppendLine("        }");
        }
        else
        {
            EmitPropertyValueWrite(sb, prop, "        ");
        }
    }

    private static void EmitNullablePrimitiveWrite(StringBuilder sb, TeaLeafProperty prop, string indent)
    {
        string name = prop.TLName;
        string access = prop.CSharpName;

        sb.AppendLine($"{indent}sb.Append(indent);");
        sb.AppendLine($"{indent}sb.Append(\"{name}: \");");

        if (prop.CSharpType.Contains("bool") || prop.CSharpType.Contains("Boolean"))
        {
            sb.AppendLine($"{indent}sb.AppendLine({access}.Value ? \"true\" : \"false\");");
        }
        else
        {
            sb.AppendLine($"{indent}sb.AppendLine({access}.Value.ToString(System.Globalization.CultureInfo.InvariantCulture));");
        }
    }

    private static void EmitPropertyValueWrite(StringBuilder sb, TeaLeafProperty prop, string indent)
    {
        string name = prop.TLName;
        string access = prop.CSharpName;

        sb.AppendLine($"{indent}sb.Append(indent);");

        switch (prop.Kind)
        {
            case PropertyKind.Primitive:
                if (prop.CSharpType.Contains("bool") || prop.CSharpType == "bool" || prop.CSharpType == "System.Boolean")
                {
                    sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                    sb.AppendLine($"{indent}sb.AppendLine({access} ? \"true\" : \"false\");");
                }
                else if (prop.CSharpType.Contains("float") || prop.CSharpType.Contains("double") ||
                         prop.CSharpType.Contains("decimal") || prop.CSharpType.Contains("Single") ||
                         prop.CSharpType.Contains("Double") || prop.CSharpType.Contains("Decimal"))
                {
                    sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                    sb.AppendLine($"{indent}sb.AppendLine({access}.ToString(System.Globalization.CultureInfo.InvariantCulture));");
                }
                else
                {
                    // Integer types
                    sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                    sb.AppendLine($"{indent}sb.AppendLine({access}.ToString(System.Globalization.CultureInfo.InvariantCulture));");
                }
                break;

            case PropertyKind.String:
                sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                sb.AppendLine($"{indent}TeaLeaf.Generators.Runtime.TLTextHelper.AppendString(sb, {access});");
                sb.AppendLine($"{indent}sb.AppendLine();");
                break;

            case PropertyKind.DateTime:
            case PropertyKind.DateTimeOffset:
                sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                if (prop.TLType == "timestamp")
                {
                    // If the type override is "timestamp" and the C# type is long/int64
                    if (prop.CSharpType.Contains("Int64") || prop.CSharpType == "long")
                    {
                        sb.AppendLine($"{indent}sb.AppendLine(System.DateTimeOffset.FromUnixTimeMilliseconds({access}).ToUniversalTime().ToString(\"yyyy-MM-ddTHH:mm:ss.fffZ\"));");
                    }
                    else if (prop.CSharpType.Contains("Int32") || prop.CSharpType == "int")
                    {
                        // int with [TLType("timestamp")] — cast to long for FromUnixTimeMilliseconds
                        sb.AppendLine($"{indent}sb.AppendLine(System.DateTimeOffset.FromUnixTimeMilliseconds((long){access}).ToUniversalTime().ToString(\"yyyy-MM-ddTHH:mm:ss.fffZ\"));");
                    }
                    else
                    {
                        sb.AppendLine($"{indent}sb.AppendLine(((System.DateTimeOffset){access}).ToUniversalTime().ToString(\"yyyy-MM-ddTHH:mm:ss.fffZ\"));");
                    }
                }
                else
                {
                    sb.AppendLine($"{indent}sb.AppendLine(((System.DateTimeOffset){access}).ToUniversalTime().ToString(\"yyyy-MM-ddTHH:mm:ss.fffZ\"));");
                }
                break;

            case PropertyKind.TimeSpan:
                sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                sb.AppendLine($"{indent}sb.AppendLine(((long){access}.TotalMilliseconds).ToString(System.Globalization.CultureInfo.InvariantCulture));");
                break;

            case PropertyKind.Guid:
                sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                sb.AppendLine($"{indent}TeaLeaf.Generators.Runtime.TLTextHelper.AppendString(sb, {access}.ToString());");
                sb.AppendLine($"{indent}sb.AppendLine();");
                break;

            case PropertyKind.Enum:
                sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                sb.AppendLine($"{indent}sb.AppendLine(TeaLeaf.Generators.Runtime.TLTextHelper.ToSnakeCase({access}.ToString()));");
                break;

            case PropertyKind.NestedObject:
                sb.AppendLine($"{indent}sb.Append(\"{name}: {{\\n\");");
                sb.AppendLine($"{indent}{access}.WriteTeaLeafObjectBody(sb, indent + \"    \");");
                sb.AppendLine($"{indent}sb.Append(indent);");
                sb.AppendLine($"{indent}sb.AppendLine(\"}}\");");
                break;

            case PropertyKind.List:
                sb.AppendLine($"{indent}sb.Append(\"{name}: [\");");
                sb.AppendLine($"{indent}var first_{prop.CSharpName} = true;");
                sb.AppendLine($"{indent}foreach (var item in {access})");
                sb.AppendLine($"{indent}{{");
                // Null guard: only for reference-type elements (nested objects, strings)
                if (prop.IsNestedTeaLeafType || IsReferenceElementType(prop.CollectionElementType))
                {
                    sb.AppendLine($"{indent}    if (item is null) continue;");
                }
                sb.AppendLine($"{indent}    if (!first_{prop.CSharpName}) sb.Append(\", \");");
                sb.AppendLine($"{indent}    first_{prop.CSharpName} = false;");
                if (prop.IsNestedTeaLeafType)
                {
                    sb.AppendLine($"{indent}    sb.Append(\"{{\\n\");");
                    sb.AppendLine($"{indent}    item.WriteTeaLeafObjectBody(sb, indent + \"        \");");
                    sb.AppendLine($"{indent}    sb.Append(indent);");
                    sb.AppendLine($"{indent}    sb.Append(\"    }}\");");
                }
                else
                {
                    sb.AppendLine($"{indent}    TeaLeaf.Generators.Runtime.TLTextHelper.AppendValue(sb, item);");
                }
                sb.AppendLine($"{indent}}}");
                sb.AppendLine($"{indent}sb.AppendLine(\"]\");");
                break;

            case PropertyKind.Dictionary:
                sb.AppendLine($"{indent}sb.AppendLine(\"{name}: {{\");");
                sb.AppendLine($"{indent}foreach (var kvp in {access})");
                sb.AppendLine($"{indent}{{");
                sb.AppendLine($"{indent}    sb.Append(indent);");
                sb.AppendLine($"{indent}    sb.Append(\"    \");");
                sb.AppendLine($"{indent}    TeaLeaf.Generators.Runtime.TLTextHelper.AppendString(sb, kvp.Key.ToString()!);");
                sb.AppendLine($"{indent}    sb.Append(\": \");");
                sb.AppendLine($"{indent}    TeaLeaf.Generators.Runtime.TLTextHelper.AppendValue(sb, kvp.Value);");
                sb.AppendLine($"{indent}    sb.AppendLine();");
                sb.AppendLine($"{indent}}}");
                sb.AppendLine($"{indent}sb.Append(indent);");
                sb.AppendLine($"{indent}sb.AppendLine(\"}}\");");
                break;

            default:
                // Fallback: convert to string
                sb.AppendLine($"{indent}sb.Append(\"{name}: \");");
                sb.AppendLine($"{indent}TeaLeaf.Generators.Runtime.TLTextHelper.AppendString(sb, {access}?.ToString() ?? \"~\");");
                sb.AppendLine($"{indent}sb.AppendLine();");
                break;
        }
    }

    private static bool IsReferenceElementType(string? elementType)
    {
        if (elementType == null) return true; // unknown → be safe, guard
        return elementType switch
        {
            "string" or "System.String" => true,
            "object" or "System.Object" => true,
            _ => false // primitives (int, long, double, bool, etc.) don't need null guard
        };
    }

    public static string GetRuntimeHelper()
    {
        return """
            // <auto-generated/>
            #nullable enable

            namespace TeaLeaf.Generators.Runtime;

            internal static class TLTextHelper
            {
                internal static void AppendString(System.Text.StringBuilder sb, string value)
                {
                    if (value.Length == 0)
                    {
                        sb.Append("\"\"");
                        return;
                    }

                    bool needsQuoting = false;
                    for (int i = 0; i < value.Length; i++)
                    {
                        char c = value[i];
                        if (c == ' ' || c == '\t' || c == '\n' || c == '\r' ||
                            c == '"' || c == '\\' || c == '#' || c == ':' ||
                            c == ',' || c == '{' || c == '}' || c == '[' ||
                            c == ']' || c == '(' || c == ')' || c == '~' ||
                            c == '@' || c == '!' || c == '/')
                        {
                            needsQuoting = true;
                            break;
                        }
                    }

                    if (!needsQuoting)
                    {
                        if (value == "true" || value == "false" || value == "~" ||
                            value == "null" ||
                            char.IsDigit(value[0]) || value[0] == '-' || value[0] == '+')
                        {
                            needsQuoting = true;
                        }
                    }

                    if (!needsQuoting)
                    {
                        sb.Append(value);
                        return;
                    }

                    sb.Append('"');
                    for (int i = 0; i < value.Length; i++)
                    {
                        char c = value[i];
                        switch (c)
                        {
                            case '\\': sb.Append("\\\\"); break;
                            case '"': sb.Append("\\\""); break;
                            case '\n': sb.Append("\\n"); break;
                            case '\t': sb.Append("\\t"); break;
                            case '\r': sb.Append("\\r"); break;
                            default: sb.Append(c); break;
                        }
                    }
                    sb.Append('"');
                }

                internal static void AppendValue<T>(System.Text.StringBuilder sb, T value)
                {
                    if (value == null)
                    {
                        sb.Append("~");
                        return;
                    }
                    string str = value.ToString()!;
                    if (value is bool b)
                    {
                        sb.Append(b ? "true" : "false");
                    }
                    else if (value is int || value is long || value is short || value is byte ||
                             value is uint || value is ulong || value is ushort || value is sbyte)
                    {
                        sb.Append(str);
                    }
                    else if (value is float || value is double || value is decimal)
                    {
                        sb.Append(((System.IFormattable)value).ToString(null, System.Globalization.CultureInfo.InvariantCulture));
                    }
                    else
                    {
                        AppendString(sb, str);
                    }
                }

                internal static string ToSnakeCase(string name)
                {
                    if (string.IsNullOrEmpty(name)) return name;
                    var sb = new System.Text.StringBuilder(name.Length + 4);
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

                internal static string FromSnakeCase(string name)
                {
                    if (string.IsNullOrEmpty(name)) return name;
                    var sb = new System.Text.StringBuilder(name.Length);
                    bool capitalizeNext = true;
                    for (int i = 0; i < name.Length; i++)
                    {
                        char c = name[i];
                        if (c == '_')
                        {
                            capitalizeNext = true;
                        }
                        else
                        {
                            sb.Append(capitalizeNext ? char.ToUpperInvariant(c) : c);
                            capitalizeNext = false;
                        }
                    }
                    return sb.ToString();
                }
            }
            """;
    }
}
