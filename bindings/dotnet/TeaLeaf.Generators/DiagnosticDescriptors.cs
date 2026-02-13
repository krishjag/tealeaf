using Microsoft.CodeAnalysis;

namespace TeaLeaf.Generators;

internal static class DiagnosticDescriptors
{
    public static readonly DiagnosticDescriptor TypeMustBePartial = new(
        id: "TL001",
        title: "TeaLeaf type must be partial",
        messageFormat: "Type '{0}' is annotated with [TeaLeaf(Generate = true)] but is not declared as partial",
        category: "TeaLeaf",
        defaultSeverity: DiagnosticSeverity.Error,
        isEnabledByDefault: true);

    public static readonly DiagnosticDescriptor UnsupportedPropertyType = new(
        id: "TL002",
        title: "Unsupported property type",
        messageFormat: "Property '{0}' on type '{1}' has unsupported type '{2}' for TeaLeaf serialization",
        category: "TeaLeaf",
        defaultSeverity: DiagnosticSeverity.Warning,
        isEnabledByDefault: true);

    public static readonly DiagnosticDescriptor InvalidTLType = new(
        id: "TL003",
        title: "Invalid TLType value",
        messageFormat: "[TLType(\"{0}\")] on property '{1}' is not a valid TeaLeaf type",
        category: "TeaLeaf",
        defaultSeverity: DiagnosticSeverity.Error,
        isEnabledByDefault: true);

    public static readonly DiagnosticDescriptor NestedTypeNotTeaLeaf = new(
        id: "TL004",
        title: "Nested type not annotated with [TeaLeaf]",
        messageFormat: "Property '{0}' references type '{1}' which is not annotated with [TeaLeaf]. It will be serialized as a plain object.",
        category: "TeaLeaf",
        defaultSeverity: DiagnosticSeverity.Info,
        isEnabledByDefault: true);

    public static readonly DiagnosticDescriptor CircularReference = new(
        id: "TL005",
        title: "Circular type reference detected",
        messageFormat: "Type '{0}' has a circular reference through '{1}'. This may cause a stack overflow at runtime.",
        category: "TeaLeaf",
        defaultSeverity: DiagnosticSeverity.Warning,
        isEnabledByDefault: true);

    public static readonly DiagnosticDescriptor OpenGenericType = new(
        id: "TL006",
        title: "Open generic type skipped",
        messageFormat: "Type '{0}' has unbound type parameters and cannot be source-generated. Use TeaLeafSerializer for generic types at runtime.",
        category: "TeaLeaf",
        defaultSeverity: DiagnosticSeverity.Info,
        isEnabledByDefault: true);

    public static readonly DiagnosticDescriptor GlobalNamespace = new(
        id: "TL007",
        title: "TeaLeaf type must be in a named namespace",
        messageFormat: "Type '{0}' is in the global namespace. [TeaLeaf] classes must be declared inside a named namespace for source generation to work correctly.",
        category: "TeaLeaf",
        defaultSeverity: DiagnosticSeverity.Error,
        isEnabledByDefault: true);
}
