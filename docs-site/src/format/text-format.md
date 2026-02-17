# Text Format

The TeaLeaf text format (`.tl`) is the human-readable representation. This page is the complete syntax reference.

## Comments

Comments begin with `#` and extend to end of line:

```tl
# This is a line comment
name: alice  # inline comment
```

Comments are stripped during compilation to binary.

## Strings

### Simple (Unquoted)

Bare identifiers that contain no whitespace or special characters:

```tl
name: alice
host: localhost
status: active
```

Valid characters: letters, digits, `_`, `-`, `.`

### Quoted

Double-quoted strings with escape sequences:

```tl
greeting: "hello world"
path: "C:\\Users\\name"
message: "line1\nline2"
tab_separated: "col1\tcol2"
```

**Escape sequences:** `\\`, `\"`, `\n`, `\t`, `\r`, `\b` (backspace), `\f` (form feed), `\uXXXX` (Unicode code point, 4 hex digits)

### Multiline (Triple-Quoted)

Triple-quoted strings with automatic leading whitespace removal:

```tl
description: """
  This is a multiline string.
  Leading whitespace is trimmed based on
  the indentation of the first content line.
  Useful for documentation blocks.
"""
```

## Numbers

### Integers

```tl
count: 42
negative: -17
zero: 0
```

### Floats

```tl
price: 3.14
scientific: 6.022e23
negative_exp: 1.5e-10
```

Numbers with exponent notation but no decimal point (e.g., `1e3`) are parsed as floats.

### Hexadecimal

```tl
color: 0xFF5500
mask: 0x00A1
```

### Binary Literals

```tl
flags: 0b1010
byte_val: 0b11110000
```

Both lowercase (`0x`, `0b`) and uppercase (`0X`, `0B`) prefixes are accepted.

Negative hex and binary literals are supported: `-0xFF`, `-0b1010`.

### Bytes Literals

```tl
payload: b"cafef00d"
empty: b""
checksum: b"CAFE"
```

Hex digits only (uppercase or lowercase), even length, no spaces.

### Special Float Values

```tl
not_a_number: NaN
positive_infinity: inf
negative_infinity: -inf
```

These keywords represent IEEE 754 special values. In JSON export, `NaN` and infinity values are converted to `null`.

## Boolean and Null

```tl
enabled: true
disabled: false
missing: ~
explicit_null: null
```

The tilde (`~`) and the `null` keyword both represent null. In most contexts they are interchangeable.

In `@table` tuples, the two forms have distinct semantics:
- **`~`** -- absent field (the field was not present in the source object). For nullable fields, the parser drops the field entirely. For non-nullable fields, it is preserved as null.
- **`null`** -- explicit null (the field was present with a null value). Always preserved regardless of field type.

## Timestamps

ISO 8601 formatted date/time values:

```tl
# Date only
created: 2024-01-15

# Date and time (UTC)
updated: 2024-01-15T10:30:00Z

# With milliseconds
precise: 2024-01-15T10:30:00.123Z

# With timezone offset
local: 2024-01-15T10:30:00+05:30
```

**Format:** `YYYY-MM-DD[THH:MM[:SS[.sss]][Z|+HH:MM|-HH:MM]]`

Seconds (`:SS`) are optional and default to `00`. Timestamps are stored internally as Unix milliseconds (`i64`).

## Objects

Curly-brace delimited key-value collections:

```tl
# Inline
point: {x: 10, y: 20}

# Multi-line
config: {
  host: localhost,
  port: 8080,
  debug: false,
}
```

Trailing commas are allowed.

## Arrays

Square-bracket delimited ordered collections:

```tl
numbers: [1, 2, 3, 4, 5]
mixed: [1, "hello", true, ~]
nested: [[1, 2], [3, 4]]
empty: []
```

## Tuples

Parenthesized value lists. Outside of `@table`, tuples are parsed as plain arrays:

```tl
# This is an array [0, 0], NOT a struct
origin: (0, 0)
```

Inside a `@table` context, tuples are bound to the table's schema:

```tl
@struct point (x: int, y: int)
points: @table point [
  (0, 0),       # bound to point schema
  (100, 200),
]
```

## Maps

Ordered key-value maps with the `@map` directive. Unlike objects, maps support non-string keys:

```tl
# String keys
headers: @map {
  "Content-Type": "application/json",
  "Accept": "*/*",
}

# Integer keys
status_codes: @map {
  200: "OK",
  404: "Not Found",
  500: "Internal Server Error",
}

# Mixed value types
config: @map {
  name: "myapp",
  port: 8080,
  debug: true,
}
```

Maps preserve insertion order and support heterogeneous key types.

## References

Define named values and reuse them:

```tl
# Define a reference
!node_a: {label: "Start", value: 1}
!node_b: {label: "End", value: 2}

# Use references
edges: [
  {from: !node_a, to: !node_b, weight: 1.0},
  {from: !node_b, to: !node_a, weight: 0.5},
]

# References can be used multiple times
nodes: [!node_a, !node_b]
```

References can be defined at the top level or inside objects.

## Tagged Values

A colon prefix adds a discriminator tag to any value:

```tl
events: [
  :click {x: 100, y: 200},
  :scroll {delta: -50},
  :keypress {key: "Enter"},
]
```

Tags are useful for discriminated unions and variant types.

## Unions

Named discriminated unions with `@union`:

```tl
@union shape {
  circle (radius: float),
  rectangle (width: float, height: float),
  point (),
}

shapes: [
  :circle (5.0),
  :rectangle (10.0, 20.0),
  :point (),
]
```

Union definitions are encoded in the binary schema table alongside struct definitions, preserving variant names, field names, and field types through compilation and decompilation.

## Root Array

The `@root-array` directive marks the document as representing a top-level JSON array. This is primarily used for JSON round-trip fidelity.

When a root-level JSON array is imported via `from-json`, TeaLeaf stores each element as a numbered key (`0`, `1`, `2`, ...) and emits `@root-array` so that `to-json` reconstructs the original array structure:

```tl
@root-array

0: {id: 1, name: alice}
1: {id: 2, name: bob}
2: {id: 3, name: carol}
```

Without `@root-array`, exporting to JSON would produce `{"0": {...}, "1": {...}, ...}`. With it, the output is `[{...}, {...}, ...]`.

The directive takes no arguments and must appear before any data pairs.

### Unknown Directives

Unknown directives (e.g., `@custom`) at the document top level are silently ignored. If a same-line argument follows the directive (e.g., `@custom foo` or `@custom [1,2,3]`), it is consumed and discarded. Arguments on the next line are not consumed — they are parsed as normal statements. This enables forward compatibility: files authored for a newer spec version can be partially parsed by older implementations that do not recognize new directives.

When an unknown directive appears as a value (e.g., `key: @unknown [1,2,3]`), it is treated as `null`. The argument expression is consumed but discarded.

## File Includes

Import other TeaLeaf files:

```tl
@include "schemas/common.tl"
@include "./shared/config.tl"
```

Paths are resolved relative to the including file. Included schemas are available for `@table` use in the including file.

## Formatting Rules

- **Trailing commas** are allowed in objects, arrays, tuples, and maps
- **Whitespace** is flexible -- indent as you like
- **Key names** follow identifier rules: start with letter or `_`, then letters, digits, `_`, `-`, `.`
- **Quoted keys** are supported for names with special characters: `"Content-Type": "application/json"`
