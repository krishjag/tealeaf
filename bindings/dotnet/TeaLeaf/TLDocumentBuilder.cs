using System.Globalization;
using System.Reflection;
using System.Text;

namespace TeaLeaf;

/// <summary>
/// Builds a multi-key TLDocument by accumulating schemas and data from multiple types.
/// Supports source-generated types (via CollectTeaLeafSchemas/WriteTeaLeafObjectBody),
/// reflection-serialized types (via TeaLeafSerializer fallback), and scalar values.
/// </summary>
public sealed class TLDocumentBuilder
{
    private readonly StringBuilder _schemas = new();
    private readonly StringBuilder _data = new();
    private readonly HashSet<string> _emittedSchemas = new();
    private readonly Dictionary<string, string> _schemaSignatures = new();

    // ----------------------------------------------------------------
    // Scalar overloads
    // ----------------------------------------------------------------

    /// <summary>
    /// Add a string value under the given key.
    /// </summary>
    public TLDocumentBuilder Add(string key, string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        _data.Append(key);
        _data.Append(": ");
        _data.AppendLine(TeaLeafTextHelper.QuoteIfNeeded(value));
        return this;
    }

    /// <summary>
    /// Add an integer value under the given key.
    /// </summary>
    public TLDocumentBuilder Add(string key, int value)
    {
        _data.Append(key);
        _data.Append(": ");
        _data.AppendLine(value.ToString(CultureInfo.InvariantCulture));
        return this;
    }

    /// <summary>
    /// Add a long value under the given key.
    /// </summary>
    public TLDocumentBuilder Add(string key, long value)
    {
        _data.Append(key);
        _data.Append(": ");
        _data.AppendLine(value.ToString(CultureInfo.InvariantCulture));
        return this;
    }

    /// <summary>
    /// Add a double value under the given key.
    /// </summary>
    public TLDocumentBuilder Add(string key, double value)
    {
        _data.Append(key);
        _data.Append(": ");
        _data.AppendLine(value.ToString(CultureInfo.InvariantCulture));
        return this;
    }

    /// <summary>
    /// Add a float value under the given key.
    /// </summary>
    public TLDocumentBuilder Add(string key, float value)
    {
        _data.Append(key);
        _data.Append(": ");
        _data.AppendLine(value.ToString(CultureInfo.InvariantCulture));
        return this;
    }

    /// <summary>
    /// Add a boolean value under the given key.
    /// </summary>
    public TLDocumentBuilder Add(string key, bool value)
    {
        _data.Append(key);
        _data.Append(": ");
        _data.AppendLine(value ? "true" : "false");
        return this;
    }

    /// <summary>
    /// Add a DateTimeOffset value under the given key.
    /// </summary>
    public TLDocumentBuilder Add(string key, DateTimeOffset value)
    {
        _data.Append(key);
        _data.Append(": ");
        _data.AppendLine(value.ToUniversalTime().ToString("o", CultureInfo.InvariantCulture));
        return this;
    }

    /// <summary>
    /// Add an array of scalar values under the given key.
    /// </summary>
    public TLDocumentBuilder AddList(string key, IEnumerable<string> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        _data.Append(key);
        _data.Append(": [");
        bool first = true;
        foreach (var item in items)
        {
            if (!first) _data.Append(", ");
            first = false;
            _data.Append(item == null ? "~" : TeaLeafTextHelper.QuoteIfNeeded(item));
        }
        _data.AppendLine("]");
        return this;
    }

    /// <summary>
    /// Add an array of integer values under the given key.
    /// </summary>
    public TLDocumentBuilder AddList(string key, IEnumerable<int> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        _data.Append(key);
        _data.Append(": [");
        bool first = true;
        foreach (var item in items)
        {
            if (!first) _data.Append(", ");
            first = false;
            _data.Append(item.ToString(CultureInfo.InvariantCulture));
        }
        _data.AppendLine("]");
        return this;
    }

    /// <summary>
    /// Add an array of double values under the given key.
    /// </summary>
    public TLDocumentBuilder AddList(string key, IEnumerable<double> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        _data.Append(key);
        _data.Append(": [");
        bool first = true;
        foreach (var item in items)
        {
            if (!first) _data.Append(", ");
            first = false;
            _data.Append(item.ToString(CultureInfo.InvariantCulture));
        }
        _data.AppendLine("]");
        return this;
    }

    // ----------------------------------------------------------------
    // Object overloads (source-generated / reflection)
    // ----------------------------------------------------------------

    /// <summary>
    /// Add a single object under the given key.
    /// </summary>
    /// <typeparam name="T">A class annotated with [TeaLeaf].</typeparam>
    /// <param name="key">The top-level key name.</param>
    /// <param name="value">The object to serialize.</param>
    /// <returns>This builder for chaining.</returns>
    public TLDocumentBuilder Add<T>(string key, T value) where T : class
    {
        ArgumentNullException.ThrowIfNull(value);

        var type = typeof(T);
        var collectMethod = type.GetMethod("CollectTeaLeafSchemas",
            BindingFlags.Public | BindingFlags.Static,
            null, new[] { typeof(StringBuilder), typeof(HashSet<string>) }, null);
        var writeBody = type.GetMethod("WriteTeaLeafObjectBody",
            BindingFlags.Public | BindingFlags.Instance,
            null, new[] { typeof(StringBuilder), typeof(string) }, null);

        if (collectMethod != null && writeBody != null)
        {
            CollectAndTrackSignatures(type, collectMethod);

            _data.Append(key);
            _data.AppendLine(": {");
            writeBody.Invoke(value, new object[] { _data, "    " });
            _data.AppendLine("}");
        }
        else
        {
            var docText = TeaLeafSerializer.ToDocument(value, key);
            _data.AppendLine(docText);
        }

        return this;
    }

    /// <summary>
    /// Add a collection of objects under the given key as a table.
    /// </summary>
    /// <typeparam name="T">A class annotated with [TeaLeaf].</typeparam>
    /// <param name="key">The top-level key name.</param>
    /// <param name="items">The collection of objects to serialize.</param>
    /// <returns>This builder for chaining.</returns>
    public TLDocumentBuilder AddList<T>(string key, IEnumerable<T> items) where T : class
    {
        ArgumentNullException.ThrowIfNull(items);

        var type = typeof(T);
        var collectMethod = type.GetMethod("CollectTeaLeafSchemas",
            BindingFlags.Public | BindingFlags.Static,
            null, new[] { typeof(StringBuilder), typeof(HashSet<string>) }, null);
        var writeBody = type.GetMethod("WriteTeaLeafObjectBody",
            BindingFlags.Public | BindingFlags.Instance,
            null, new[] { typeof(StringBuilder), typeof(string) }, null);

        if (collectMethod != null && writeBody != null)
        {
            CollectAndTrackSignatures(type, collectMethod);

            _data.Append(key);
            _data.AppendLine(": [");
            foreach (var item in items)
            {
                if (item == null) continue;
                _data.AppendLine("    {");
                writeBody.Invoke(item, new object[] { _data, "        " });
                _data.AppendLine("    }");
            }
            _data.AppendLine("]");
        }
        else
        {
            var docText = TeaLeafSerializer.ToText(items, key);
            _data.AppendLine(docText);
        }

        return this;
    }

    /// <summary>
    /// Merge an existing TLDocument's content into this builder.
    /// Schemas are extracted and deduplicated; data is appended without schemas
    /// to prevent order-dependent schema collisions.
    /// </summary>
    /// <param name="doc">The document to merge.</param>
    /// <returns>This builder for chaining.</returns>
    /// <exception cref="InvalidOperationException">
    /// Thrown when a schema name conflicts with a previously added schema that has a different field layout.
    /// </exception>
    public TLDocumentBuilder AddDocument(TLDocument doc)
    {
        ArgumentNullException.ThrowIfNull(doc);

        // Extract schemas with dedup into _schemas
        foreach (var schema in doc.Schemas)
        {
            var sig = SchemaSignature(schema);

            if (!_emittedSchemas.Add(schema.Name))
            {
                // Conflict detection: compare field shapes
                if (_schemaSignatures.TryGetValue(schema.Name, out var existing)
                    && existing != sig)
                    throw new InvalidOperationException(
                        $"Schema '{schema.Name}' conflicts with a previously added schema of the same name but different field layout.");
            }
            else
            {
                _schemaSignatures[schema.Name] = sig;
                if (_schemas.Length > 0) _schemas.AppendLine();
                _schemas.Append($"@struct {schema.Name} (");
                for (int i = 0; i < schema.Fields.Count; i++)
                {
                    if (i > 0) _schemas.Append(", ");
                    var field = schema.Fields[i];
                    if (field.IsArray) _schemas.Append("[]");
                    _schemas.Append($"{QuoteFieldName(field.Name)}: {field.Type}");
                    if (field.IsNullable) _schemas.Append('?');
                }
                _schemas.Append(')');
            }
        }

        // Append data only (no schemas) to avoid order-dependent collisions
        _data.AppendLine(doc.ToText(ignoreSchemas: true));
        return this;
    }

    /// <summary>
    /// Build the accumulated content into a TLDocument.
    /// </summary>
    /// <returns>A parsed TLDocument containing all added keys and schemas. The caller must dispose.</returns>
    /// <exception cref="TLException">Thrown if the accumulated text cannot be parsed.</exception>
    public TLDocument Build()
    {
        var sb = new StringBuilder(_schemas.Length + _data.Length + 4);
        if (_schemas.Length > 0)
        {
            sb.AppendLine(_schemas.ToString());
            sb.AppendLine();
        }
        sb.Append(_data);
        return TLDocument.Parse(sb.ToString());
    }

    /// <summary>
    /// Invokes CollectTeaLeafSchemas and back-fills _schemaSignatures for any
    /// newly emitted schemas so that later AddDocument calls can detect conflicts.
    /// </summary>
    private void CollectAndTrackSignatures(Type type, MethodInfo collectMethod)
    {
        var before = _schemas.Length;
        collectMethod.Invoke(null, new object[] { _schemas, _emittedSchemas });

        // If new schema text was appended, parse it to extract signatures
        if (_schemas.Length > before)
        {
            var newText = _schemas.ToString(before, _schemas.Length - before);
            using var tempDoc = TLDocument.Parse(newText);
            foreach (var schema in tempDoc.Schemas)
            {
                if (!_schemaSignatures.ContainsKey(schema.Name))
                    _schemaSignatures[schema.Name] = SchemaSignature(schema);
            }
        }
    }

    /// <summary>
    /// Produces a canonical signature for a schema's field layout, used for conflict detection.
    /// </summary>
    private static string SchemaSignature(TLSchema schema) =>
        string.Join(",", schema.Fields.Select(f =>
            $"{(f.IsArray ? "[]" : "")}{f.Name}:{f.Type}{(f.IsNullable ? "?" : "")}"));

    /// <summary>
    /// Quotes a field name with double quotes if it contains characters that
    /// require quoting in @struct definitions. Escapes embedded double quotes.
    /// </summary>
    private static string QuoteFieldName(string name)
    {
        if (name.Length == 0) return "\"\"";

        bool needsQuoting = false;
        foreach (var c in name)
        {
            if (!char.IsLetterOrDigit(c) && c != '_' && c != '-')
            {
                needsQuoting = true;
                break;
            }
        }

        if (!needsQuoting) return name;
        return $"\"{name.Replace("\\", "\\\\").Replace("\"", "\\\"")}\"";
    }
}
