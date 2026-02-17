# Schemas

Schemas are the foundation of TeaLeaf's compact encoding. They define structure once so data can use positional encoding.

## Defining Schemas

Use `@struct` to define a schema:

```tl
@struct point (x: int, y: int)
```

With multiple fields and types:

```tl
@struct user (
  id: int,
  name: string,
  email: string?,
  active: bool,
)
```

### Optional Type Annotations

Field types can be omitted -- they default to `string`:

```tl
@struct config (host, port: int, debug: bool)
# host defaults to string type
```

## Using Schemas with @table

The `@table` directive binds an array of tuples to a schema:

```tl
@struct user (id: int, name: string, email: string)

users: @table user [
  (1, "Alice", "alice@example.com"),
  (2, "Bob", "bob@example.com"),
  (3, "Carol", "carol@example.com"),
]
```

Each tuple's values are matched **positionally** to the schema fields.

## Nested Structs

Structs can reference other structs. Nested tuples inherit schema binding from their parent field type:

```tl
@struct address (street: string, city: string, zip: string)

@struct person (
  name: string,
  home: address,
  work: address?,
)

people: @table person [
  (
    "Alice Smith",
    ("123 Main St", "Berlin", "10115"),     # Parsed as address
    ("456 Office Blvd", "Berlin", "10117"), # Parsed as address
  ),
]
```

## Deep Nesting

Schemas can nest arbitrarily deep:

```tl
@struct method (type: string, last_four: string)
@struct payment (amount: float, method: method)
@struct order (id: int, customer: string, payment: payment)

orders: @table order [
  (1, "Alice", (99.99, ("credit", "4242"))),
  (2, "Bob", (49.50, ("debit", "1234"))),
]
```

## Array Fields

Schema fields can be arrays of primitives or other structs:

```tl
@struct employee (
  id: int,
  name: string,
  skills: []string,
  scores: []int,
)

employees: @table employee [
  (1, "Alice", ["rust", "python"], [95, 88]),
  (2, "Bob", ["java"], [72]),
]
```

## Nullable Fields

The `?` modifier makes a field nullable:

```tl
@struct user (
  id: int,
  name: string,
  email: string?,   # can be ~ or null
  phone: string?,   # can be ~ or null
)

users: @table user [
  (1, "Alice", "alice@example.com", "+1-555-0100"),
  (2, "Bob", ~, ~),           # ~ = absent (fields dropped from output)
  (3, "Carol", null, ~),      # null = explicit null (email preserved as null)
]
```

In `@table` tuples, `~` and `null` have different meanings:
- **`~`** -- absent field. For nullable fields, the field is dropped entirely from the reconstructed object. For non-nullable fields, it is preserved as null.
- **`null`** -- explicit null. The field was present with a null value in the source data. Always preserved as `null` in the output, regardless of field nullability.

This distinction ensures JSON round-trip fidelity: `{"email": null}` roundtrips as `null` (preserved), while a missing `email` key roundtrips as `~` (dropped).

## Binary Encoding Benefits

Schemas enable significant binary compression:

1. **Positional storage** -- field names stored once in the schema table, not per row
2. **Two-bit field state bitmaps** -- two bits per field per row track has-value, explicit null, and absent states
3. **Type-homogeneous arrays** -- packed encoding when all elements match a schema
4. **String deduplication** -- repeated values like city names stored once in the string table

### Example Size Savings

For 1,000 user records with 5 fields:

| Approach | Approximate Size |
|---|---|
| JSON (field names repeated) | ~80KB |
| TeaLeaf text (schema + tuples) | ~35KB |
| TeaLeaf binary (compressed) | ~15KB |

## Schema Compatibility

### Compatible Changes

| Change | Notes |
|--------|-------|
| Rename field | Data is positional; names are documentation only |
| Widen type | `int8` → `int64`, `float32` → `float64` (automatic) |

### Incompatible Changes (Require Recompile)

| Change | Resolution |
|--------|-----------|
| Add field | Recompile source `.tl` file |
| Remove field | Recompile source `.tl` file |
| Reorder fields | Recompile source `.tl` file |
| Narrow type | Recompile source `.tl` file |

### Recompilation Workflow

The `.tl` file is the master. When schemas change:

```bash
tealeaf compile data.tl -o data.tlbx
```

TeaLeaf prioritizes simplicity over automatic schema evolution:
- **No migration machinery** -- recompile when schemas change
- **No version negotiation** -- the embedded schema is the source of truth
- **Explicit over implicit** -- tuples require values for all fields
