using System.Text;

namespace TeaLeaf.Generators;

internal static class DeserializerEmitter
{
    public static void EmitDeserializationMethods(StringBuilder sb, TeaLeafModel model)
    {
        string key = model.DefaultKey;

        // FromTeaLeaf(TLDocument)
        sb.AppendLine($"    /// <summary>Deserializes a <see cref=\"{model.TypeName}\"/> from a TLDocument by key.</summary>");
        sb.AppendLine($"    /// <param name=\"doc\">The TLDocument to read from.</param>");
        sb.AppendLine($"    /// <param name=\"key\">The top-level key to look up. Defaults to the struct name.</param>");
        sb.AppendLine($"    /// <returns>A deserialized <see cref=\"{model.TypeName}\"/> instance.</returns>");
        sb.AppendLine($"    /// <exception cref=\"TeaLeaf.TLException\">Thrown if the key is not found in the document.</exception>");
        sb.AppendLine($"    public static {model.TypeName} FromTeaLeaf(TeaLeaf.TLDocument doc, string key = \"{key}\")");
        sb.AppendLine("    {");
        sb.AppendLine("        using var value = doc.Get(key)");
        sb.AppendLine($"            ?? throw new TeaLeaf.TLException($\"Key '{{key}}' not found in document\");");
        sb.AppendLine("        return FromTeaLeaf(value);");
        sb.AppendLine("    }");
        sb.AppendLine();

        // FromTeaLeaf(TLValue)
        sb.AppendLine($"    /// <summary>Deserializes a <see cref=\"{model.TypeName}\"/> from a TLValue.</summary>");
        sb.AppendLine($"    /// <param name=\"value\">A TLValue of type Object containing the serialized fields.</param>");
        sb.AppendLine($"    /// <returns>A deserialized <see cref=\"{model.TypeName}\"/> instance.</returns>");
        sb.AppendLine($"    /// <exception cref=\"TeaLeaf.TLException\">Thrown if the value is not of type Object.</exception>");
        sb.AppendLine($"    public static {model.TypeName} FromTeaLeaf(TeaLeaf.TLValue value)");
        sb.AppendLine("    {");
        sb.AppendLine($"        if (value.Type != TeaLeaf.TLType.Object)");
        sb.AppendLine($"            throw new TeaLeaf.TLException(");
        sb.AppendLine($"                $\"Expected Object, got {{value.Type}} when deserializing {model.TypeName}\");");
        sb.AppendLine();
        sb.AppendLine($"        var result = new {model.TypeName}();");
        sb.AppendLine();

        foreach (var prop in model.Properties)
        {
            if (prop.IsSkipped) continue;
            EmitPropertyRead(sb, prop, model.TypeName);
        }

        sb.AppendLine("        return result;");
        sb.AppendLine("    }");
    }

    private static void EmitPropertyRead(StringBuilder sb, TeaLeafProperty prop, string typeName)
    {
        string tlName = prop.TLName;
        string csharpName = prop.CSharpName;

        sb.AppendLine($"        using (var f_{csharpName} = value.GetField(\"{tlName}\"))");
        sb.AppendLine("        {");

        switch (prop.Kind)
        {
            case PropertyKind.Primitive:
                EmitPrimitiveRead(sb, prop);
                break;

            case PropertyKind.String:
                if (prop.IsNullable)
                {
                    sb.AppendLine($"            if (f_{csharpName} is not null && !f_{csharpName}.IsNull)");
                    sb.AppendLine($"                result.{csharpName} = f_{csharpName}.AsString();");
                }
                else
                {
                    sb.AppendLine($"            if (f_{csharpName} is not null)");
                    sb.AppendLine($"                result.{csharpName} = f_{csharpName}.AsString() ?? \"\";");
                }
                break;

            case PropertyKind.DateTime:
            case PropertyKind.DateTimeOffset:
                if (prop.TLType == "timestamp" && (prop.CSharpType.Contains("Int64") || prop.CSharpType == "long"))
                {
                    // long with [TLType("timestamp")]
                    sb.AppendLine($"            if (f_{csharpName} is not null)");
                    sb.AppendLine($"                result.{csharpName} = f_{csharpName}.AsTimestamp() ?? f_{csharpName}.AsInt() ?? 0;");
                }
                else if (prop.TLType == "timestamp" && (prop.CSharpType.Contains("Int32") || prop.CSharpType == "int"))
                {
                    // int with [TLType("timestamp")]
                    sb.AppendLine($"            if (f_{csharpName} is not null)");
                    sb.AppendLine($"                result.{csharpName} = (int)(f_{csharpName}.AsTimestamp() ?? f_{csharpName}.AsInt() ?? 0);");
                }
                else
                {
                    sb.AppendLine($"            if (f_{csharpName} is not null && !f_{csharpName}.IsNull)");
                    sb.AppendLine($"                result.{csharpName} = f_{csharpName}.AsDateTime() ?? System.DateTimeOffset.MinValue;");
                }
                break;

            case PropertyKind.TimeSpan:
                sb.AppendLine($"            if (f_{csharpName} is not null)");
                sb.AppendLine($"                result.{csharpName} = System.TimeSpan.FromMilliseconds(f_{csharpName}.AsInt() ?? 0);");
                break;

            case PropertyKind.Guid:
                sb.AppendLine($"            if (f_{csharpName} is not null)");
                sb.AppendLine($"            {{");
                sb.AppendLine($"                var str = f_{csharpName}.AsString();");
                sb.AppendLine($"                if (str != null) result.{csharpName} = System.Guid.Parse(str);");
                sb.AppendLine($"            }}");
                break;

            case PropertyKind.Enum:
                sb.AppendLine($"            if (f_{csharpName} is not null)");
                sb.AppendLine($"            {{");
                sb.AppendLine($"                var enumStr = f_{csharpName}.AsString();");
                sb.AppendLine($"                if (enumStr != null)");
                sb.AppendLine($"                {{");
                // Convert snake_case to PascalCase for enum parsing
                sb.AppendLine($"                    var pascalCase = TeaLeaf.Generators.Runtime.TLTextHelper.FromSnakeCase(enumStr);");
                sb.AppendLine($"                    if (System.Enum.TryParse<{prop.CSharpType}>(pascalCase, true, out var enumVal))");
                sb.AppendLine($"                        result.{csharpName} = enumVal;");
                sb.AppendLine($"                }}");
                sb.AppendLine($"            }}");
                break;

            case PropertyKind.NestedObject:
                if (prop.IsNullable)
                {
                    sb.AppendLine($"            if (f_{csharpName} is not null && !f_{csharpName}.IsNull)");
                    sb.AppendLine($"                result.{csharpName} = {prop.CSharpType.TrimEnd('?')}.FromTeaLeaf(f_{csharpName});");
                }
                else
                {
                    sb.AppendLine($"            if (f_{csharpName} is not null)");
                    sb.AppendLine($"                result.{csharpName} = {prop.CSharpType}.FromTeaLeaf(f_{csharpName});");
                }
                break;

            case PropertyKind.List:
                EmitListRead(sb, prop);
                break;

            case PropertyKind.Dictionary:
                EmitDictionaryRead(sb, prop);
                break;

            default:
                // Fallback: try string
                sb.AppendLine($"            if (f_{csharpName} is not null)");
                sb.AppendLine($"                result.{csharpName} = f_{csharpName}.AsString() ?? \"\";");
                break;
        }

        sb.AppendLine("        }");
        sb.AppendLine();
    }

    private static void EmitPrimitiveRead(StringBuilder sb, TeaLeafProperty prop)
    {
        string csharpName = prop.CSharpName;
        string baseType = prop.CSharpType.TrimEnd('?');

        string accessor = baseType switch
        {
            "bool" or "System.Boolean" => $"f_{csharpName}.AsBool()",
            "byte" or "System.Byte" => $"(byte?)(f_{csharpName}.AsInt())",
            "sbyte" or "System.SByte" => $"(sbyte?)(f_{csharpName}.AsInt())",
            "short" or "System.Int16" => $"(short?)(f_{csharpName}.AsInt())",
            "ushort" or "System.UInt16" => $"(ushort?)(f_{csharpName}.AsUInt())",
            "int" or "System.Int32" => $"(int?)(f_{csharpName}.AsInt())",
            "uint" or "System.UInt32" => $"(uint?)(f_{csharpName}.AsUInt())",
            "long" or "System.Int64" => $"f_{csharpName}.AsInt()",
            "ulong" or "System.UInt64" => $"f_{csharpName}.AsUInt()",
            "float" or "System.Single" => $"(float?)(f_{csharpName}.AsFloat() ?? (double?)f_{csharpName}.AsInt())",
            "double" or "System.Double" => $"(f_{csharpName}.AsFloat() ?? (double?)f_{csharpName}.AsInt())",
            "decimal" or "System.Decimal" => $"(decimal?)(f_{csharpName}.AsFloat() ?? (double?)f_{csharpName}.AsInt())",
            _ => $"f_{csharpName}.AsInt()"
        };

        string defaultVal = baseType switch
        {
            "bool" or "System.Boolean" => "false",
            _ => "0"
        };

        if (prop.IsCSharpNullable)
        {
            // C# type is nullable (int?, bool?, etc.) — assign nullable accessor directly
            sb.AppendLine($"            if (f_{csharpName} is not null && !f_{csharpName}.IsNull)");
            sb.AppendLine($"                result.{csharpName} = {accessor};");
        }
        else
        {
            // C# type is non-nullable — use default value fallback
            sb.AppendLine($"            if (f_{csharpName} is not null)");
            sb.AppendLine($"                result.{csharpName} = {accessor} ?? {defaultVal};");
        }
    }

    private static void EmitListRead(StringBuilder sb, TeaLeafProperty prop)
    {
        string csharpName = prop.CSharpName;
        string elemType = prop.CollectionElementType ?? "string";

        sb.AppendLine($"            if (f_{csharpName} is not null && f_{csharpName}.Type == TeaLeaf.TLType.Array)");
        sb.AppendLine("            {");
        sb.AppendLine($"                var list = new System.Collections.Generic.List<{elemType}>();");
        sb.AppendLine($"                for (int i = 0; i < f_{csharpName}.ArrayLength; i++)");
        sb.AppendLine("                {");
        sb.AppendLine($"                    using var elem = f_{csharpName}.GetArrayElement(i);");
        sb.AppendLine("                    if (elem is not null)");
        sb.AppendLine("                    {");

        if (prop.IsNestedTeaLeafType)
        {
            sb.AppendLine($"                        list.Add({elemType}.FromTeaLeaf(elem));");
        }
        else
        {
            // Determine how to read the element
            string elemRead = GetElementReadExpression(elemType, "elem");
            sb.AppendLine($"                        {elemRead}");
        }

        sb.AppendLine("                    }");
        sb.AppendLine("                }");
        sb.AppendLine($"                result.{csharpName} = list;");
        sb.AppendLine("            }");
    }

    private static string GetElementReadExpression(string elemType, string varName)
    {
        return elemType switch
        {
            "string" => $"var s = {varName}.AsString(); if (s != null) list.Add(s);",
            "int" or "System.Int32" => $"list.Add((int)({varName}.AsInt() ?? 0));",
            "long" or "System.Int64" => $"list.Add({varName}.AsInt() ?? 0);",
            "double" or "System.Double" => $"list.Add({varName}.AsFloat() ?? (double?)({varName}.AsInt()) ?? 0.0);",
            "float" or "System.Single" => $"list.Add((float)({varName}.AsFloat() ?? (double?)({varName}.AsInt()) ?? 0.0));",
            "bool" or "System.Boolean" => $"list.Add({varName}.AsBool() ?? false);",
            _ => $"var s = {varName}.AsString(); if (s != null) list.Add(s);",
        };
    }

    private static void EmitDictionaryRead(StringBuilder sb, TeaLeafProperty prop)
    {
        string csharpName = prop.CSharpName;
        string valueType = prop.CollectionElementType ?? "string";

        sb.AppendLine($"            if (f_{csharpName} is not null && f_{csharpName}.Type == TeaLeaf.TLType.Object)");
        sb.AppendLine("            {");
        sb.AppendLine($"                var dict = new System.Collections.Generic.Dictionary<string, {valueType}>();");
        sb.AppendLine($"                var keys = f_{csharpName}.ObjectKeys;");
        sb.AppendLine($"                foreach (var k in keys)");
        sb.AppendLine("                {");
        sb.AppendLine($"                    using var v = f_{csharpName}[k];");

        string valueRead = GetDictionaryValueReadExpression(valueType, "v");
        sb.AppendLine($"                    {valueRead}");

        sb.AppendLine("                }");
        sb.AppendLine($"                result.{csharpName} = dict;");
        sb.AppendLine("            }");
    }

    private static string GetDictionaryValueReadExpression(string valueType, string varName)
    {
        return valueType switch
        {
            "string" => $"dict[k] = {varName}?.AsString() ?? \"\";",
            "int" or "System.Int32" => $"dict[k] = (int)({varName}?.AsInt() ?? 0);",
            "long" or "System.Int64" => $"dict[k] = {varName}?.AsInt() ?? 0;",
            "double" or "System.Double" => $"dict[k] = {varName}?.AsFloat() ?? (double?)({varName}?.AsInt()) ?? 0.0;",
            "float" or "System.Single" => $"dict[k] = (float)({varName}?.AsFloat() ?? (double?)({varName}?.AsInt()) ?? 0.0);",
            "bool" or "System.Boolean" => $"dict[k] = {varName}?.AsBool() ?? false;",
            _ => $"dict[k] = {varName}?.AsString() ?? \"\";",
        };
    }
}
