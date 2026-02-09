using TeaLeaf;

if (args.Length < 2 || args[0] != "--root")
{
    Console.Error.WriteLine("Usage: AdversarialHarness --root <repo-root>");
    return 2;
}

var repoRoot = args[1];
var inputs = Path.Combine(repoRoot, "adversarial-tests", "inputs");

var cases = new List<(string Name, Action Action, bool ExpectFailure)>
{
    (
        "parse_invalid_unclosed_string",
        () => TLDocument.Parse("name: \"Alice"),
        true
    ),
    (
        "parse_invalid_escape",
        () => TLDocument.Parse("name: \"Alice\\q\""),
        true
    ),
    (
        "parse_unicode_escape_short",
        () => TLDocument.Parse("text: \"\\u12\""),
        true
    ),
    (
        "parse_unicode_escape_invalid_hex",
        () => TLDocument.Parse("text: \"\\uZZZZ\""),
        true
    ),
    (
        "parse_unicode_escape_surrogate",
        () => TLDocument.Parse("text: \"\\uD800\""),
        true
    ),
    (
        "parse_unterminated_multiline",
        () => TLDocument.Parse("text: \"\"\"unterminated"),
        true
    ),
    (
        "parse_missing_colon",
        () => TLDocument.Parse("name \"Alice\""),
        true
    ),
    (
        "parse_file_invalid",
        () => TLDocument.ParseFile(Path.Combine(inputs, "tl", "bad_unclosed_string.tl")),
        true
    ),
    (
        "parse_file_invalid_utf8",
        () => TLDocument.ParseFile(Path.Combine(inputs, "tl", "invalid_utf8.tl")),
        true
    ),
    (
        "from_json_invalid",
        () => TLDocument.FromJson("{\"a\":1,}"),
        true
    ),
    (
        "from_json_large_number_overflow",
        () =>
        {
            using var doc = TLDocument.FromJson("{\"big\": 18446744073709551616}");
            var value = doc.Get("big");
            if (value == null || value.Type != TLType.Float)
            {
                throw new Exception("big is not float");
            }
        },
        false
    ),
    (
        "from_json_root_array",
        () =>
        {
            using var doc = TLDocument.FromJson("[1,2,3]");
            if (!doc.ContainsKey("root"))
            {
                throw new Exception("missing root");
            }
        },
        false
    ),
    (
        "parse_deep_nesting_ok",
        () => {
            using var doc = TLDocument.Parse("root: [[[[[[[1]]]]]]]");
            if (!doc.ContainsKey("root"))
            {
                throw new Exception("missing root");
            }
        },
        false
    ),
};

var failures = new List<string>();

foreach (var (name, action, expectFailure) in cases)
{
    var failed = false;
    try
    {
        action();
        if (expectFailure)
        {
            failed = true;
        }
    }
    catch
    {
        if (!expectFailure)
        {
            failed = true;
        }
    }

    if (failed)
    {
        failures.Add(name);
    }
}

if (failures.Count > 0)
{
    Console.Error.WriteLine("Adversarial harness failures:");
    foreach (var failure in failures)
    {
        Console.Error.WriteLine($"- {failure}");
    }
    return 1;
}

Console.WriteLine("Adversarial harness passed.");
return 0;
