# Core Concepts

This page introduces the fundamental ideas behind TeaLeaf.

## Dual Format

TeaLeaf has two representations of the same data:

| | Text (`.tl`) | Binary (`.tlbx`) |
|---|---|---|
| **Purpose** | Authoring, version control, review | Storage, transmission, deployment |
| **Human-readable** | Yes | No |
| **Comments** | Yes (`#`) | Stripped during compilation |
| **Schemas** | Inline `@struct` definitions | Embedded in schema table |
| **Size** | Larger (field names in data) | Compact (positional, deduplicated) |
| **Speed** | Slower to parse | Fast random-access via memory mapping |

The `.tl` file is the **source of truth**. Binary files are compiled artifacts — regenerate them when the source changes.

## Schemas

Schemas define the structure of your data using `@struct`:

```tl
@struct point (x: int, y: int)
@struct line (start: point, end: point, color: string?)
```

Key properties:

- **Inline** — schemas live in the same file as data
- **Positional** — binary encoding uses field order, not names
- **Nestable** — structs can reference other structs
- **Nullable** — fields marked with `?` accept null (`~`)

Schemas enable `@table` for compact tabular data:

```tl
points: @table point [
  (0, 0),
  (100, 200),
  (-50, 75),
]
```

Without schemas, the same data would require repeating field names:

```tl
# Without schemas — verbose
points: [
  {x: 0, y: 0},
  {x: 100, y: 200},
  {x: -50, y: 75},
]
```

## Type System

TeaLeaf has a rich type system with primitives, containers, and modifiers.

### Primitives

| Type | Description | Example |
|------|-------------|---------|
| `bool` | Boolean | `true`, `false` |
| `int` / `int32` | 32-bit signed integer | `42`, `-17` |
| `int64` | 64-bit signed integer | `9999999999` |
| `uint` / `uint32` | 32-bit unsigned integer | `255` |
| `float` / `float64` | 64-bit float | `3.14`, `6.022e23` |
| `string` | UTF-8 text | `"hello"`, `alice` |
| `bytes` | Raw binary data | (binary format only) |
| `timestamp` | ISO 8601 date/time | `2024-01-15T10:30:00Z` |

### Containers

| Syntax | Description |
|--------|-------------|
| `[]T` | Array of type T |
| `T?` | Nullable type T |
| `@map { ... }` | Ordered key-value map |
| `{ key: value }` | Untyped object |

### Null

The tilde `~` represents null:

```tl
optional_field: ~
```

## Key-Value Documents

A TeaLeaf document is a collection of named key-value sections:

```tl
# Each top-level entry is a "section" in the binary format
config: {host: localhost, port: 8080}
users: @table user [(1, alice), (2, bob)]
version: "2.0.0"
```

Keys become section names in the binary file. You access values by key at runtime.

## References

References allow data reuse and graph structures:

```tl
# Define a reference
!seattle: {city: "Seattle", state: "WA"}

# Use it in multiple places
office: !seattle
warehouse: !seattle
```

## Tagged Values

Tags add a discriminator label to values, enabling sum types:

```tl
events: [
  :click {x: 100, y: 200},
  :scroll {delta: -50},
  :keypress {key: "Enter"},
]
```

## Unions

Named discriminated unions:

```tl
@union shape {
  circle (radius: float),
  rectangle (width: float, height: float),
  point (),
}

shapes: [:circle (5.0), :rectangle (10.0, 20.0), :point ()]
```

> **Note:** Unions are parsed but not fully implemented in binary encoding yet. Use tagged values for runtime-safe discriminated data.

## Compilation Pipeline

```
   .tl (text)
      │
      ├── parse ──> in-memory document (TeaLeaf / TLDocument)
      │                    │
      │                    ├── compile ──> .tlbx (binary)
      │                    ├── to_json ──> .json
      │                    └── to_tl_text ──> .tl (round-trip)
      │
   .tlbx (binary)
      │
      ├── reader ──> random-access values (zero-copy with mmap)
      │                    │
      │                    ├── decompile ──> .tl
      │                    └── to_json ──> .json
      │
   .json
      │
      └── from_json ──> in-memory document
                             │
                             └── (with schema inference for arrays)
```

## File Includes

Split large files into modules:

```tl
@include "schemas/common.tl"
@include "./data/users.tl"
```

Paths are resolved relative to the including file.

## Next Steps

- [Text Format](../format/text-format.md) — complete syntax reference
- [Type System](../format/type-system.md) — all types and modifiers in detail
- [Schemas](../format/schemas.md) — schema definitions, tables, and nesting
