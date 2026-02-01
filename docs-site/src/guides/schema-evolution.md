# Schema Evolution

TeaLeaf takes a deliberately simple approach to schema evolution: when schemas change, recompile.

## Design Philosophy

- **No migration machinery** — no schema versioning or compatibility negotiation
- **Source file is master** — the `.tl` file defines the current schema
- **Explicit over implicit** — tuples require values for all fields
- **Binary is a compiled artifact** — regenerate it like you would a compiled binary

## Compatible Changes

These changes do **not** require recompilation of existing binary files:

### Rename Fields

Field data is stored positionally. Names are documentation only:

```tl
# Before
@struct user (name: string, email: string)

# After — binary still works
@struct user (full_name: string, email_address: string)
```

### Widen Types

Automatic safe widening when reading:

```tl
# Before: field was int8
@struct sensor (id: int8, reading: float32)

# After: widened to int32 — readers auto-widen
@struct sensor (id: int, reading: float)
```

Widening path: `int8` → `int16` → `int32` → `int64`, `float32` → `float64`

## Incompatible Changes

These changes **require recompilation** from the `.tl` source:

### Add a Field

```tl
# Before
@struct user (id: int, name: string)

# After — added email field
@struct user (id: int, name: string, email: string?)
```

Old binary files won't have the new field. Recompile:

```bash
tealeaf compile users.tl -o users.tlbx
```

### Remove a Field

```tl
# Before
@struct user (id: int, name: string, legacy_field: string)

# After — removed legacy_field
@struct user (id: int, name: string)
```

### Reorder Fields

Binary data is positional. Changing field order changes the meaning of stored data:

```tl
# Before
@struct point (x: int, y: int)

# After — DON'T DO THIS without recompiling
@struct point (y: int, x: int)
```

### Narrow Types

Narrowing (e.g., `int64` → `int8`) can lose data:

```tl
# Before
@struct data (value: int64)

# After — potential data loss
@struct data (value: int8)
```

## Recompilation Workflow

When schemas change:

```bash
# 1. Edit the .tl source file
# 2. Validate
tealeaf validate data.tl

# 3. Recompile
tealeaf compile data.tl -o data.tlbx

# 4. Verify
tealeaf info data.tlbx
```

## Migration Strategy

For applications that need to handle schema changes:

### Approach 1: Version Keys

Use different top-level keys for different schema versions:

```tl
@struct user_v1 (id: int, name: string)
@struct user_v2 (id: int, name: string, email: string?, role: string)

# Old data
users_v1: @table user_v1 [(1, alice), (2, bob)]

# New data
users_v2: @table user_v2 [(3, carol, "carol@ex.com", admin)]
```

### Approach 2: Application-Level Migration

Read old binary, transform in code, write new binary:

```rust
// Read old format
let old_doc = TeaLeaf::load("data_v1.tlbx")?;

// Transform
let new_doc = TeaLeafBuilder::new()
    .add_vec("users", &migrate_users(old_doc.get("users")))
    .build();

// Write new format
new_doc.compile("data_v2.tlbx", true)?;
```

### Approach 3: Nullable Fields

Add new fields as nullable to maintain backward compatibility:

```tl
@struct user (
  id: int,
  name: string,
  email: string?,    # new field, nullable
  phone: string?,    # new field, nullable
)
```

Old data can have `~` for new fields. New data populates them.

## Comparison with Other Formats

| Aspect | TeaLeaf | Protobuf | Avro |
|--------|---------|----------|------|
| Schema location | Inline in data file | External `.proto` | Embedded in binary |
| Adding fields | Recompile | Compatible (field numbers) | Compatible (defaults) |
| Removing fields | Recompile | Compatible (skip unknown) | Compatible (skip) |
| Migration tool | None (recompile) | protoc | Schema registry |
| Complexity | Low | Medium | High |

TeaLeaf trades automatic evolution for simplicity. If your use case requires frequent schema changes across distributed systems, consider Protobuf or Avro.
