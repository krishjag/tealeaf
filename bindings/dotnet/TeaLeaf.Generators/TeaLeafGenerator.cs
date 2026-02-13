using System.Collections.Immutable;
using System.Text;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Text;

namespace TeaLeaf.Generators;

[Generator]
public class TeaLeafGenerator : IIncrementalGenerator
{
    public void Initialize(IncrementalGeneratorInitializationContext context)
    {
        // Register the runtime helper (TLTextHelper)
        context.RegisterPostInitializationOutput(ctx =>
        {
            ctx.AddSource("TLTextHelper.g.cs", SourceText.From(TLTextEmitter.GetRuntimeHelper(), Encoding.UTF8));
        });

        // Collect all types with [TeaLeaf] attribute
        var allTeaLeafTypes = context.SyntaxProvider
            .ForAttributeWithMetadataName(
                "TeaLeaf.Annotations.TeaLeafAttribute",
                predicate: static (node, _) => node is TypeDeclarationSyntax,
                transform: static (ctx, ct) => GetResult(ctx, ct));

        // Report diagnostics for open generic types (TL006)
        var openGenericTypes = allTeaLeafTypes
            .Where(static r => r.IsOpenGeneric)
            .Select(static (r, _) => r.DiagnosticLocation!);

        context.RegisterSourceOutput(openGenericTypes, static (spc, loc) =>
        {
            spc.ReportDiagnostic(Diagnostic.Create(
                DiagnosticDescriptors.OpenGenericType,
                loc.Location,
                loc.TypeName));
        });

        // Report diagnostics for non-partial types (TL001)
        var nonPartialTypes = allTeaLeafTypes
            .Where(static r => r.IsNonPartial)
            .Select(static (r, _) => r.DiagnosticLocation!);

        context.RegisterSourceOutput(nonPartialTypes, static (spc, loc) =>
        {
            spc.ReportDiagnostic(Diagnostic.Create(
                DiagnosticDescriptors.TypeMustBePartial,
                loc.Location,
                loc.TypeName));
        });

        // Report diagnostics for global namespace types (TL007)
        var globalNamespaceTypes = allTeaLeafTypes
            .Where(static r => r.IsGlobalNamespace)
            .Select(static (r, _) => r.DiagnosticLocation!);

        context.RegisterSourceOutput(globalNamespaceTypes, static (spc, loc) =>
        {
            spc.ReportDiagnostic(Diagnostic.Create(
                DiagnosticDescriptors.GlobalNamespace,
                loc.Location,
                loc.TypeName));
        });

        // Generate source for concrete (non-generic) types
        var concreteModels = allTeaLeafTypes
            .Where(static r => r.Model is not null)
            .Select(static (r, _) => r.Model!);

        context.RegisterSourceOutput(concreteModels, static (spc, model) =>
        {
            // Report property-level diagnostics (TL002, TL003, TL004)
            ReportPropertyDiagnostics(spc, model);

            var source = GenerateSource(model);
            spc.AddSource($"{model.TypeName}.TeaLeaf.g.cs",
                SourceText.From(source, Encoding.UTF8));
        });
    }

    private static GeneratorResult GetResult(
        GeneratorAttributeSyntaxContext context,
        System.Threading.CancellationToken ct)
    {
        if (context.TargetSymbol is not INamedTypeSymbol typeSymbol)
            return default;

        // Check Generate property — skip if not opted in to source generation.
        // [TeaLeaf] alone is for reflection-based serialization; [TeaLeaf(Generate = true)] opts in.
        var tealeafAttr = typeSymbol.GetAttributes()
            .FirstOrDefault(a => a.AttributeClass?.Name == "TeaLeafAttribute");
        bool generate = false;
        if (tealeafAttr != null)
        {
            foreach (var namedArg in tealeafAttr.NamedArguments)
            {
                if (namedArg.Key == "Generate" && namedArg.Value.Value is bool g)
                    generate = g;
            }
        }
        if (!generate)
            return default;

        var location = new DiagnosticLocationInfo(
            typeSymbol.Name,
            context.TargetNode.GetLocation());

        // Skip open generic types — they cannot be source-generated.
        // Users should use TeaLeafSerializer for generic types at runtime.
        if (typeSymbol.TypeParameters.Length > 0)
        {
            return new GeneratorResult(
                isOpenGeneric: true,
                diagnosticLocation: location);
        }

        // Check if the type declaration is partial (TL001)
        if (context.TargetNode is TypeDeclarationSyntax typeDecl &&
            !typeDecl.Modifiers.Any(SyntaxKind.PartialKeyword))
        {
            return new GeneratorResult(
                isNonPartial: true,
                diagnosticLocation: location);
        }

        // Check if the type is in the global namespace (TL007)
        if (typeSymbol.ContainingNamespace is null or { IsGlobalNamespace: true })
        {
            return new GeneratorResult(
                isGlobalNamespace: true,
                diagnosticLocation: location);
        }

        return new GeneratorResult(model: ModelAnalyzer.Analyze(typeSymbol));
    }

    private static void ReportPropertyDiagnostics(SourceProductionContext spc, TeaLeafModel model)
    {
        foreach (var prop in model.Properties)
        {
            if (prop.IsSkipped) continue;

            // TL002: Unsupported property type
            if (prop.Kind == PropertyKind.Unknown)
            {
                spc.ReportDiagnostic(Diagnostic.Create(
                    DiagnosticDescriptors.UnsupportedPropertyType,
                    Location.None,
                    prop.CSharpName, model.TypeName, prop.CSharpType));
            }

            // TL003: Invalid TLType value
            if (!string.IsNullOrEmpty(prop.TLTypeOverride) &&
                !ModelAnalyzer.IsValidTLType(prop.TLTypeOverride!))
            {
                spc.ReportDiagnostic(Diagnostic.Create(
                    DiagnosticDescriptors.InvalidTLType,
                    Location.None,
                    prop.TLTypeOverride, prop.CSharpName));
            }

            // TL004: Nested type not annotated with [TeaLeaf]
            if (prop.Kind == PropertyKind.Unknown && prop.IsClassType)
            {
                spc.ReportDiagnostic(Diagnostic.Create(
                    DiagnosticDescriptors.NestedTypeNotTeaLeaf,
                    Location.None,
                    prop.CSharpName, prop.CSharpType));
            }
        }
    }

    private readonly struct GeneratorResult
    {
        public TeaLeafModel? Model { get; }
        public bool IsOpenGeneric { get; }
        public bool IsNonPartial { get; }
        public bool IsGlobalNamespace { get; }
        public DiagnosticLocationInfo? DiagnosticLocation { get; }

        public GeneratorResult(
            TeaLeafModel? model = null,
            bool isOpenGeneric = false,
            bool isNonPartial = false,
            bool isGlobalNamespace = false,
            DiagnosticLocationInfo? diagnosticLocation = null)
        {
            Model = model;
            IsOpenGeneric = isOpenGeneric;
            IsNonPartial = isNonPartial;
            IsGlobalNamespace = isGlobalNamespace;
            DiagnosticLocation = diagnosticLocation;
        }
    }

    private sealed class DiagnosticLocationInfo
    {
        public string TypeName { get; }
        public Location Location { get; }

        public DiagnosticLocationInfo(string typeName, Location location)
        {
            TypeName = typeName;
            Location = location;
        }
    }

    private static string GenerateSource(TeaLeafModel model)
    {
        var sb = new StringBuilder(4096);
        sb.AppendLine("// <auto-generated/>");
        sb.AppendLine("#nullable enable");
        sb.AppendLine();

        if (!string.IsNullOrEmpty(model.Namespace))
        {
            sb.AppendLine($"namespace {model.Namespace};");
            sb.AppendLine();
        }

        sb.AppendLine($"partial class {model.TypeName}");
        sb.AppendLine("{");

        // Serialization methods
        TLTextEmitter.EmitSerializationMethods(sb, model);

        sb.AppendLine();

        // Deserialization methods
        DeserializerEmitter.EmitDeserializationMethods(sb, model);

        sb.AppendLine("}");

        return sb.ToString();
    }
}
