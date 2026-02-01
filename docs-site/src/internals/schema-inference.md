# Schema Inference

TeaLeaf can automatically infer schemas from JSON arrays of uniform objects. This page explains the algorithm.

## When Schema Inference Runs

Schema inference is triggered by:
- `tealeaf from-json` CLI command
- `tealeaf json-to-tlbx` CLI command
- `TeaLeaf::from_json_with_schemas()` Rust API

It is **not** triggered by:
- `TeaLeaf::from_json()` (plain import, no schemas)
- `TLDocument.FromJson()` (.NET API -- plain import)

## Algorithm

### Step 1: Array Detection

Scan top-level JSON values for arrays where **all** elements are objects with identical key sets:

```json
{
  "users": [           // ← Candidate: array of uniform objects
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"}
  ],
  "tags": ["a", "b"],  // ← Not candidate: array of strings
  "config": {...}       // ← Not candidate: not an array
}
```

### Step 2: Name Inference

The schema name is derived from the parent key by singularization:

| Key | Inferred Schema Name |
|-----|---------------------|
| `"users"` | `user` |
| `"products"` | `product` |
| `"employees"` | `employee` |
| `"addresses"` | `address` |
| `"data"` | `data` (already singular) |
| `"items_list"` | `items_list` (compound, kept as-is) |

Basic singularization rules:
- Remove trailing `s` if the word doesn't end in `ss`
- Remove trailing `es` for `-es` words
- Remove trailing `ies` → `y`

### Step 3: Type Inference

For each field, scan all array elements to determine the type:

| JSON Values Seen | Inferred TeaLeaf Type |
|-----------------|----------------------|
| All integers | `int` |
| All numbers (mixed int/float) | `float` |
| All strings | `string` |
| All booleans | `bool` |
| All objects (uniform keys) | Nested struct reference |
| All arrays | Inferred element type |
| Mixed types | `string` (fallback) |

### Step 4: Nullable Detection

If any element has `null` for a field, that field becomes nullable:

```json
[
  {"id": 1, "email": "alice@ex.com"},
  {"id": 2, "email": null}           // ← email becomes string?
]
```

### Step 5: Nested Schema Inference

If a field's value is an object across all array elements, and those objects have identical keys, a nested schema is created:

```json
{
  "users": [
    {"name": "Alice", "address": {"city": "Seattle", "zip": "98101"}},
    {"name": "Bob", "address": {"city": "Austin", "zip": "78701"}}
  ]
}
```

Inferred schemas:
```tl
@struct address (city: string, zip: string)
@struct user (address: address, name: string)
```

This is recursive -- nested objects can have their own nested schemas.

## Output

The inferred schemas are:
1. Added to the document as `@struct` definitions
2. The original JSON arrays are converted to `@table` tuples
3. Written in the output file before the data

### Example

**Input JSON:**
```json
{
  "products": [
    {"id": 1, "name": "Widget", "price": 9.99, "in_stock": true},
    {"id": 2, "name": "Gadget", "price": 24.99, "in_stock": false}
  ]
}
```

**Output TeaLeaf:**
```tl
@struct product (id: int, in_stock: bool, name: string, price: float)

products: @table product [
  (1, true, Widget, 9.99),
  (2, false, Gadget, 24.99),
]
```

## Limitations

1. **Field order** -- JSON objects have no guaranteed order. Fields are sorted alphabetically in the inferred schema.

2. **Type ambiguity** -- JSON numbers don't distinguish int from float. If any element has a decimal, the field becomes `float`.

3. **Non-uniform arrays** -- arrays where objects have different key sets are not schema-inferred. They remain as plain arrays of objects.

4. **Deeply nested arrays** -- only the first level of array → schema inference is applied. Nested arrays within objects are not auto-inferred.

5. **No timestamp detection** -- ISO 8601 strings in JSON remain as strings, not timestamps.
