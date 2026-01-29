# Canonical test: Special types available in text format
# Tests: references, tagged values, maps, edge cases

# Define a reference (can be reused multiple times)
!base_config: {
    theme: dark
    language: en
    debug: false
}

# Use the reference
config_ref: !base_config

# Tagged values
result_ok: :ok 200
result_error: :error {code: 404, message: "Not found"}

# Maps with @map directive
headers: @map {
    "Content-Type": "application/json",
    "Accept": "*/*",
}

status_codes: @map {
    200: "OK",
    404: "Not Found",
    500: "Internal Server Error",
}

# Inline objects that look like special types (but are plain objects)
# These should NOT be converted to special types when exported to JSON and re-imported
pseudo_ref: {"$ref": some_key}
pseudo_tag: {"$tag": ok, "$value": 200}

# Various edge cases
empty_string: ""
whitespace_string: "   "
multiline_value: "line one
line two
line three"
special_chars: "tab:\there\nquote:\"end"
