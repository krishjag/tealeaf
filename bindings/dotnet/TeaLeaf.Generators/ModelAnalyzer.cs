using Microsoft.CodeAnalysis;

namespace TeaLeaf.Generators;

internal sealed class TeaLeafModel
{
    public string Namespace { get; set; } = "";
    public string TypeName { get; set; } = "";
    public string FullyQualifiedName { get; set; } = "";
    public string StructName { get; set; } = "";
    public bool IsRecord { get; set; }
    public bool EmitSchema { get; set; } = true;
    public string DefaultKey { get; set; } = "";
    public List<TeaLeafProperty> Properties { get; set; } = new();
    public List<string> NestedTeaLeafTypeNames { get; set; } = new();
}

internal sealed class TeaLeafProperty
{
    public string CSharpName { get; set; } = "";
    public string TLName { get; set; } = "";
    public string TLType { get; set; } = "";
    public string CSharpType { get; set; } = "";
    public bool IsNullable { get; set; }
    public bool IsCSharpNullable { get; set; }
    public bool IsOptional { get; set; }
    public bool IsSkipped { get; set; }
    public bool IsCollection { get; set; }
    public bool IsEnum { get; set; }
    public bool IsNestedTeaLeafType { get; set; }
    public string? CollectionElementType { get; set; }
    public string? NestedTeaLeafStructName { get; set; }
    public PropertyKind Kind { get; set; }
    /// <summary>The raw [TLType] override value, if any. Used for diagnostic validation.</summary>
    public string? TLTypeOverride { get; set; }
    /// <summary>True if the property type is a class/interface (not primitive, not enum, not collection).</summary>
    public bool IsClassType { get; set; }
}

internal enum PropertyKind
{
    Primitive,
    String,
    DateTime,
    DateTimeOffset,
    ByteArray,
    Enum,
    NestedObject,
    List,
    Dictionary,
    Guid,
    TimeSpan,
    Unknown,
}

internal static class ModelAnalyzer
{
    private static readonly HashSet<string> ValidTLTypes = new()
    {
        "bool", "int", "int8", "int16", "int32", "int64",
        "uint", "uint8", "uint16", "uint32", "uint64",
        "float", "float32", "float64", "string", "bytes", "timestamp"
    };

    public static TeaLeafModel? Analyze(INamedTypeSymbol typeSymbol)
    {
        var tealeafAttr = typeSymbol.GetAttributes()
            .FirstOrDefault(a => a.AttributeClass?.Name == "TeaLeafAttribute");
        if (tealeafAttr == null) return null;

        string? structNameOverride = null;
        bool emitSchema = true;

        foreach (var namedArg in tealeafAttr.NamedArguments)
        {
            if (namedArg.Key == "StructName" && namedArg.Value.Value is string sn)
                structNameOverride = sn;
            if (namedArg.Key == "EmitSchema" && namedArg.Value.Value is bool es)
                emitSchema = es;
        }

        string typeName = typeSymbol.Name;
        string structName = structNameOverride ?? ToSnakeCase(typeName);

        // Check for [TLKey] attribute
        string defaultKey = structName;
        var keyAttr = typeSymbol.GetAttributes()
            .FirstOrDefault(a => a.AttributeClass?.Name == "TLKeyAttribute");
        if (keyAttr?.ConstructorArguments.Length > 0 && keyAttr.ConstructorArguments[0].Value is string keyVal)
            defaultKey = keyVal;

        var properties = new List<TeaLeafProperty>();
        var nestedTypes = new List<string>();

        foreach (var member in typeSymbol.GetMembers())
        {
            if (member is not IPropertySymbol prop) continue;
            if (prop.IsStatic || prop.IsIndexer) continue;
            if (prop.DeclaredAccessibility != Accessibility.Public) continue;

            var tlProp = AnalyzeProperty(prop, nestedTypes);
            if (tlProp != null)
                properties.Add(tlProp);
        }

        string ns = typeSymbol.ContainingNamespace is { IsGlobalNamespace: false } nsSymbol
            ? nsSymbol.ToDisplayString()
            : "";
        string fqn = typeSymbol.ToDisplayString();

        return new TeaLeafModel
        {
            Namespace = ns,
            TypeName = typeName,
            FullyQualifiedName = fqn,
            StructName = structName,
            IsRecord = typeSymbol.IsRecord,
            EmitSchema = emitSchema,
            DefaultKey = defaultKey,
            Properties = properties,
            NestedTeaLeafTypeNames = nestedTypes,
        };
    }

    private static TeaLeafProperty? AnalyzeProperty(IPropertySymbol prop, List<string> nestedTypes)
    {
        // Check for [TLSkip]
        bool isSkipped = prop.GetAttributes().Any(a => a.AttributeClass?.Name == "TLSkipAttribute");

        // Check for [TLRename]
        string? rename = null;
        var renameAttr = prop.GetAttributes()
            .FirstOrDefault(a => a.AttributeClass?.Name == "TLRenameAttribute");
        if (renameAttr?.ConstructorArguments.Length > 0 && renameAttr.ConstructorArguments[0].Value is string rn)
            rename = rn;

        // Check for [TLOptional]
        bool isOptional = prop.GetAttributes().Any(a => a.AttributeClass?.Name == "TLOptionalAttribute");

        // Check for [TLType]
        string? typeOverride = null;
        var typeAttr = prop.GetAttributes()
            .FirstOrDefault(a => a.AttributeClass?.Name == "TLTypeAttribute");
        if (typeAttr?.ConstructorArguments.Length > 0 && typeAttr.ConstructorArguments[0].Value is string tn)
            typeOverride = tn;

        string csharpName = prop.Name;
        string tlName = rename ?? ToSnakeCase(csharpName);
        string csharpType = prop.Type.ToDisplayString();

        bool isNullable = prop.Type.NullableAnnotation == NullableAnnotation.Annotated
                          || (prop.Type is INamedTypeSymbol nts && nts.OriginalDefinition.SpecialType == SpecialType.System_Nullable_T);

        var (kind, tlType, isCollection, elementType, isEnum, isNestedTL, nestedStructName) =
            ClassifyType(prop.Type, typeOverride);

        if (isNestedTL && nestedStructName != null && !nestedTypes.Contains(nestedStructName))
            nestedTypes.Add(nestedStructName);

        return new TeaLeafProperty
        {
            CSharpName = csharpName,
            TLName = tlName,
            TLType = typeOverride ?? tlType,
            CSharpType = csharpType,
            IsNullable = isNullable || isOptional,
            IsCSharpNullable = isNullable,
            IsOptional = isOptional,
            IsSkipped = isSkipped,
            IsCollection = isCollection,
            IsEnum = isEnum,
            IsNestedTeaLeafType = isNestedTL,
            CollectionElementType = elementType,
            NestedTeaLeafStructName = nestedStructName,
            Kind = kind,
            TLTypeOverride = typeOverride,
            IsClassType = prop.Type.TypeKind == TypeKind.Class || prop.Type.TypeKind == TypeKind.Interface,
        };
    }

    private static (PropertyKind kind, string tlType, bool isCollection, string? elementType,
        bool isEnum, bool isNestedTL, string? nestedStructName)
        ClassifyType(ITypeSymbol type, string? typeOverride)
    {
        // Unwrap Nullable<T>
        if (type is INamedTypeSymbol { OriginalDefinition.SpecialType: SpecialType.System_Nullable_T } nullable)
        {
            var inner = nullable.TypeArguments[0];
            var (k, t, c, e, en, n, ns) = ClassifyType(inner, typeOverride);
            return (k, t, c, e, en, n, ns);
        }

        if (typeOverride == "timestamp")
            return (PropertyKind.DateTimeOffset, "timestamp", false, null, false, false, null);

        var specialType = type.SpecialType;
        return specialType switch
        {
            SpecialType.System_Boolean => (PropertyKind.Primitive, "bool", false, null, false, false, null),
            SpecialType.System_Byte => (PropertyKind.Primitive, "uint8", false, null, false, false, null),
            SpecialType.System_SByte => (PropertyKind.Primitive, "int8", false, null, false, false, null),
            SpecialType.System_Int16 => (PropertyKind.Primitive, "int16", false, null, false, false, null),
            SpecialType.System_UInt16 => (PropertyKind.Primitive, "uint16", false, null, false, false, null),
            SpecialType.System_Int32 => (PropertyKind.Primitive, "int", false, null, false, false, null),
            SpecialType.System_UInt32 => (PropertyKind.Primitive, "uint", false, null, false, false, null),
            SpecialType.System_Int64 => (PropertyKind.Primitive, "int64", false, null, false, false, null),
            SpecialType.System_UInt64 => (PropertyKind.Primitive, "uint64", false, null, false, false, null),
            SpecialType.System_Single => (PropertyKind.Primitive, "float32", false, null, false, false, null),
            SpecialType.System_Double => (PropertyKind.Primitive, "float", false, null, false, false, null),
            SpecialType.System_Decimal => (PropertyKind.Primitive, "float", false, null, false, false, null),
            SpecialType.System_String => (PropertyKind.String, "string", false, null, false, false, null),
            _ => ClassifyNonPrimitive(type)
        };
    }

    private static (PropertyKind kind, string tlType, bool isCollection, string? elementType,
        bool isEnum, bool isNestedTL, string? nestedStructName)
        ClassifyNonPrimitive(ITypeSymbol type)
    {
        string typeName = type.ToDisplayString();

        // DateTime / DateTimeOffset
        if (typeName == "System.DateTime" || typeName == "System.DateTimeOffset")
            return (PropertyKind.DateTimeOffset, "timestamp", false, null, false, false, null);

        // TimeSpan
        if (typeName == "System.TimeSpan")
            return (PropertyKind.TimeSpan, "int64", false, null, false, false, null);

        // Guid
        if (typeName == "System.Guid")
            return (PropertyKind.Guid, "string", false, null, false, false, null);

        // byte[]
        if (type is IArrayTypeSymbol { ElementType.SpecialType: SpecialType.System_Byte })
            return (PropertyKind.ByteArray, "bytes", false, null, false, false, null);

        // Enum
        if (type.TypeKind == TypeKind.Enum)
            return (PropertyKind.Enum, "string", false, null, true, false, null);

        // List<T>, IList<T>, IReadOnlyList<T>, T[]
        if (type is IArrayTypeSymbol arrayType)
        {
            string elemType = GetTLTypeForElement(arrayType.ElementType);
            bool isNestedElement = HasTeaLeafAttribute(arrayType.ElementType);
            string? nestedName = isNestedElement ? ToSnakeCase(arrayType.ElementType.Name) : null;
            return (PropertyKind.List, $"[]{elemType}", true, arrayType.ElementType.ToDisplayString(),
                false, isNestedElement, nestedName);
        }

        if (type is INamedTypeSymbol namedType)
        {
            // List<T>, IList<T>, etc.
            if (IsListLike(namedType))
            {
                var elemSymbol = namedType.TypeArguments[0];
                string elemType = GetTLTypeForElement(elemSymbol);
                bool isNestedElement = HasTeaLeafAttribute(elemSymbol);
                string? nestedName = isNestedElement ? ToSnakeCase(elemSymbol.Name) : null;
                return (PropertyKind.List, $"[]{elemType}", true, elemSymbol.ToDisplayString(),
                    false, isNestedElement, nestedName);
            }

            // Dictionary<string, T>
            if (IsDictionaryLike(namedType) && namedType.TypeArguments.Length == 2)
            {
                var valueSymbol = namedType.TypeArguments[1];
                return (PropertyKind.Dictionary, "object", false, valueSymbol.ToDisplayString(), false, false, null);
            }

            // Nested [TeaLeaf] type
            if (HasTeaLeafAttribute(namedType))
            {
                string structName = ToSnakeCase(namedType.Name);
                return (PropertyKind.NestedObject, structName, false, null, false, true, structName);
            }
        }

        return (PropertyKind.Unknown, "string", false, null, false, false, null);
    }

    private static bool IsListLike(INamedTypeSymbol type)
    {
        string name = type.OriginalDefinition.ToDisplayString();
        return name.StartsWith("System.Collections.Generic.List<")
               || name.StartsWith("System.Collections.Generic.IList<")
               || name.StartsWith("System.Collections.Generic.IReadOnlyList<")
               || name.StartsWith("System.Collections.Generic.IEnumerable<")
               || name.StartsWith("System.Collections.Generic.ICollection<")
               || name.StartsWith("System.Collections.Generic.IReadOnlyCollection<");
    }

    private static bool IsDictionaryLike(INamedTypeSymbol type)
    {
        string name = type.OriginalDefinition.ToDisplayString();
        return name.StartsWith("System.Collections.Generic.Dictionary<")
               || name.StartsWith("System.Collections.Generic.IDictionary<");
    }

    private static bool HasTeaLeafAttribute(ITypeSymbol type)
    {
        return type.GetAttributes().Any(a => a.AttributeClass?.Name == "TeaLeafAttribute");
    }

    private static string GetTLTypeForElement(ITypeSymbol type)
    {
        return type.SpecialType switch
        {
            SpecialType.System_Boolean => "bool",
            SpecialType.System_Int32 => "int",
            SpecialType.System_Int64 => "int64",
            SpecialType.System_Double => "float",
            SpecialType.System_String => "string",
            _ => HasTeaLeafAttribute(type) ? ToSnakeCase(type.Name) : "string"
        };
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

    internal static bool IsValidTLType(string typeName) => ValidTLTypes.Contains(typeName);
}
