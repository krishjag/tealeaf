using System.Collections.Concurrent;
using System.Reflection;
using TeaLeaf.Annotations;

namespace TeaLeaf.Internal;

/// <summary>
/// Cached metadata about a TeaLeaf-annotated type, built via reflection.
/// </summary>
internal sealed class TeaLeafTypeInfo
{
    private static readonly ConcurrentDictionary<Type, TeaLeafTypeInfo> Cache = new();

    public string StructName { get; }
    public string Key { get; }
    public TeaLeafPropertyInfo[] Properties { get; }
    public ConstructorInfo? ParameterizedConstructor { get; }
    public ConstructorParamMapping[]? ConstructorParamMappings { get; }
    /// <summary>
    /// True when the type has no parameterless constructor AND the parameterized constructor
    /// has unmatched parameters (no corresponding property and no default value).
    /// In this case, use RuntimeHelpers.GetUninitializedObject() + property setters.
    /// </summary>
    public bool UseUninitializedObject { get; }

    private TeaLeafTypeInfo(string structName, string key, TeaLeafPropertyInfo[] properties,
        ConstructorInfo? parameterizedConstructor = null, ConstructorParamMapping[]? constructorParamMappings = null,
        bool useUninitializedObject = false)
    {
        StructName = structName;
        Key = key;
        Properties = properties;
        ParameterizedConstructor = parameterizedConstructor;
        ConstructorParamMappings = constructorParamMappings;
        UseUninitializedObject = useUninitializedObject;
    }

    /// <summary>
    /// Gets or creates cached TeaLeafTypeInfo for the given type.
    /// </summary>
    public static TeaLeafTypeInfo GetOrCreate(Type type)
    {
        return Cache.GetOrAdd(type, static t => Build(t));
    }

    private static TeaLeafTypeInfo Build(Type type)
    {
        var teaLeafAttr = type.GetCustomAttribute<TeaLeafAttribute>();
        var structName = teaLeafAttr?.StructName ?? TeaLeafTextHelper.ToSnakeCase(type.Name);

        // Handle generic type names: strip backtick suffix (e.g., "Wrapper`1" -> "wrapper")
        if (structName.Contains('`'))
            structName = TeaLeafTextHelper.ToSnakeCase(type.Name.Substring(0, type.Name.IndexOf('`')));

        var keyAttr = type.GetCustomAttribute<TLKeyAttribute>();
        var key = keyAttr?.Key ?? structName;

        // Check for parameterless constructor
        bool hasParameterlessCtor = type.GetConstructor(
            BindingFlags.Public | BindingFlags.Instance, null, Type.EmptyTypes, null) != null;

        // Collect all readable public non-skipped properties
        var allReadableProps = new List<PropertyInfo>();
        foreach (var pi in type.GetProperties(BindingFlags.Public | BindingFlags.Instance))
        {
            if (!pi.CanRead) continue;
            if (pi.GetIndexParameters().Length > 0) continue; // skip indexers
            if (pi.GetCustomAttribute<TLSkipAttribute>() != null) continue;
            allReadableProps.Add(pi);
        }

        // Resolve parameterized constructor when no parameterless one exists
        ConstructorInfo? parameterizedCtor = null;
        ConstructorParamMapping[]? ctorMappings = null;
        var ctorParamPropertyNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        if (!hasParameterlessCtor)
        {
            var ctors = type.GetConstructors(BindingFlags.Public | BindingFlags.Instance);
            parameterizedCtor = ctors.OrderByDescending(c => c.GetParameters().Length).FirstOrDefault();

            if (parameterizedCtor != null)
            {
                var ctorParams = parameterizedCtor.GetParameters();
                ctorMappings = new ConstructorParamMapping[ctorParams.Length];

                for (int i = 0; i < ctorParams.Length; i++)
                {
                    var param = ctorParams[i];
                    // Match constructor param name to property name (case-insensitive)
                    var matchedProp = allReadableProps.FirstOrDefault(p =>
                        string.Equals(p.Name, param.Name, StringComparison.OrdinalIgnoreCase));

                    string tlName;
                    if (matchedProp != null)
                    {
                        var renameAttr = matchedProp.GetCustomAttribute<TLRenameAttribute>();
                        tlName = renameAttr?.Name ?? TeaLeafTextHelper.ToSnakeCase(matchedProp.Name);
                        ctorParamPropertyNames.Add(matchedProp.Name);
                    }
                    else
                    {
                        tlName = TeaLeafTextHelper.ToSnakeCase(param.Name!);
                    }

                    ctorMappings[i] = new ConstructorParamMapping(
                        tlName, param.ParameterType, matchedProp?.Name,
                        param.HasDefaultValue, param.HasDefaultValue ? param.DefaultValue : null);
                }

                // Reject constructor if any parameter is unmatched AND has no default value.
                // Such constructors have logic that can't be reproduced from serialized data alone.
                bool allParamsSatisfiable = ctorMappings.All(m =>
                    m.MatchedPropertyName != null || m.HasDefaultValue);
                if (!allParamsSatisfiable)
                {
                    parameterizedCtor = null;
                    ctorMappings = null;
                    ctorParamPropertyNames.Clear();
                }
            }
        }

        bool useUninitializedObject = !hasParameterlessCtor && parameterizedCtor == null;

        // Build property list
        var props = new List<TeaLeafPropertyInfo>();
        foreach (var pi in allReadableProps)
        {
            // Include property if it has a setter OR if it matches a constructor parameter
            bool isCtorParam = ctorParamPropertyNames.Contains(pi.Name);
            if (!pi.CanWrite && !isCtorParam) continue;

            var renameAttr = pi.GetCustomAttribute<TLRenameAttribute>();
            var tlName = renameAttr?.Name ?? TeaLeafTextHelper.ToSnakeCase(pi.Name);

            var typeAttr = pi.GetCustomAttribute<TLTypeAttribute>();
            var optionalAttr = pi.GetCustomAttribute<TLOptionalAttribute>();

            var propertyType = pi.PropertyType;
            bool isNullableRef = IsNullableReferenceType(pi);
            bool isNullableValue = Nullable.GetUnderlyingType(propertyType) != null;
            bool isNullable = isNullableRef || isNullableValue || optionalAttr != null;

            var tlType = typeAttr?.TypeName ?? InferTLType(propertyType, isNullable);

            bool isList = false;
            bool isDictionary = false;
            bool isEnum = false;
            bool isNestedTeaLeaf = false;
            Type? elementType = null;

            var underlyingType = Nullable.GetUnderlyingType(propertyType) ?? propertyType;

            if (underlyingType.IsGenericType)
            {
                var genDef = underlyingType.GetGenericTypeDefinition();
                if (genDef == typeof(List<>))
                {
                    isList = true;
                    elementType = underlyingType.GetGenericArguments()[0];
                }
                else if (genDef == typeof(Dictionary<,>))
                {
                    isDictionary = true;
                    elementType = underlyingType.GetGenericArguments()[1];
                }
            }

            if (underlyingType.IsEnum)
            {
                isEnum = true;
            }
            else if (!isList && !isDictionary && !IsPrimitive(underlyingType))
            {
                isNestedTeaLeaf = underlyingType.GetCustomAttribute<TeaLeafAttribute>() != null;
            }

            var getter = CreateGetter(pi);
            var setter = pi.CanWrite ? CreateSetter(pi) : null;

            props.Add(new TeaLeafPropertyInfo(
                cSharpName: pi.Name,
                tlName: tlName,
                tlType: tlType,
                propertyType: propertyType,
                isNullable: isNullable,
                isList: isList,
                isDictionary: isDictionary,
                isEnum: isEnum,
                isNestedTeaLeaf: isNestedTeaLeaf,
                elementType: elementType,
                getter: getter,
                setter: setter));
        }

        return new TeaLeafTypeInfo(structName, key, props.ToArray(), parameterizedCtor, ctorMappings, useUninitializedObject);
    }

    private static string InferTLType(Type type, bool isNullable)
    {
        var baseName = TeaLeafTextHelper.GetTLTypeName(type);
        if (isNullable && !baseName.EndsWith("?"))
            return baseName + "?";
        return baseName;
    }

    private static bool IsPrimitive(Type type)
    {
        var t = Nullable.GetUnderlyingType(type) ?? type;
        return t == typeof(bool) || t == typeof(int) || t == typeof(long) ||
               t == typeof(short) || t == typeof(byte) || t == typeof(sbyte) ||
               t == typeof(uint) || t == typeof(ulong) || t == typeof(ushort) ||
               t == typeof(double) || t == typeof(float) || t == typeof(decimal) ||
               t == typeof(string) || t == typeof(byte[]) ||
               t == typeof(DateTime) || t == typeof(DateTimeOffset);
    }

    private static bool IsNullableReferenceType(PropertyInfo pi)
    {
        // Check NullableAttribute on property (emitted by the compiler for nullable reference types)
        var nullableAttr = pi.GetCustomAttributesData()
            .FirstOrDefault(a => a.AttributeType.FullName == "System.Runtime.CompilerServices.NullableAttribute");

        if (nullableAttr != null)
        {
            var args = nullableAttr.ConstructorArguments;
            if (args.Count > 0)
            {
                if (args[0].Value is byte b)
                    return b == 2; // 2 = nullable
                if (args[0].Value is System.Collections.ObjectModel.ReadOnlyCollection<CustomAttributeTypedArgument> arr
                    && arr.Count > 0 && arr[0].Value is byte firstByte)
                    return firstByte == 2;
            }
        }

        // For reference types without explicit annotation, check the NullableContextAttribute on the declaring type
        if (!pi.PropertyType.IsValueType)
        {
            var contextAttr = pi.DeclaringType?.GetCustomAttributesData()
                .FirstOrDefault(a => a.AttributeType.FullName == "System.Runtime.CompilerServices.NullableContextAttribute");
            if (contextAttr != null && contextAttr.ConstructorArguments.Count > 0
                && contextAttr.ConstructorArguments[0].Value is byte ctx)
            {
                return ctx == 2; // 2 = nullable context
            }
        }

        return false;
    }

    private static Func<object, object?> CreateGetter(PropertyInfo pi)
    {
        return obj => pi.GetValue(obj);
    }

    private static Action<object, object?> CreateSetter(PropertyInfo pi)
    {
        return (obj, val) => pi.SetValue(obj, val);
    }
}

/// <summary>
/// Metadata about a single property in a TeaLeaf-annotated type.
/// </summary>
internal sealed class TeaLeafPropertyInfo
{
    public string CSharpName { get; }
    public string TLName { get; }
    public string TLType { get; }
    public Type PropertyType { get; }
    public bool IsNullable { get; }
    public bool IsList { get; }
    public bool IsDictionary { get; }
    public bool IsEnum { get; }
    public bool IsNestedTeaLeaf { get; }
    public Type? ElementType { get; }
    public Func<object, object?> Getter { get; }
    public Action<object, object?>? Setter { get; }

    public TeaLeafPropertyInfo(
        string cSharpName,
        string tlName,
        string tlType,
        Type propertyType,
        bool isNullable,
        bool isList,
        bool isDictionary,
        bool isEnum,
        bool isNestedTeaLeaf,
        Type? elementType,
        Func<object, object?> getter,
        Action<object, object?>? setter)
    {
        CSharpName = cSharpName;
        TLName = tlName;
        TLType = tlType;
        PropertyType = propertyType;
        IsNullable = isNullable;
        IsList = isList;
        IsDictionary = isDictionary;
        IsEnum = isEnum;
        IsNestedTeaLeaf = isNestedTeaLeaf;
        ElementType = elementType;
        Getter = getter;
        Setter = setter;
    }
}

/// <summary>
/// Maps a constructor parameter to its TeaLeaf field name and type.
/// </summary>
internal sealed class ConstructorParamMapping
{
    public string TLName { get; }
    public Type ParameterType { get; }
    public string? MatchedPropertyName { get; }
    public bool HasDefaultValue { get; }
    public object? DefaultValue { get; }

    public ConstructorParamMapping(string tlName, Type parameterType, string? matchedPropertyName,
        bool hasDefaultValue, object? defaultValue)
    {
        TLName = tlName;
        ParameterType = parameterType;
        MatchedPropertyName = matchedPropertyName;
        HasDefaultValue = hasDefaultValue;
        DefaultValue = defaultValue;
    }
}
