# Attributes Reference

All attributes use the `#[tealeaf(...)]` namespace and can be applied to structs, enums, or individual fields.

## Container Attributes

Applied to a struct or enum:

### `rename = "name"`

Override the schema name used in TeaLeaf output:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "app_config")]
struct Config {
    host: String,
    port: i32,
}
// Generates: @struct app_config (host: string, port: int)
```

Without `rename`, the struct name is converted to `snake_case` (`Config` → `config`).

### `key = "name"`

Override the default document key when serializing:

```rust
#[derive(ToTeaLeaf)]
#[tealeaf(key = "my_config")]
struct Config { /* ... */ }
```

### `root_array`

Mark a struct as a root-level array element (changes serialization to omit the wrapping key):

```rust
#[derive(ToTeaLeaf)]
#[tealeaf(root_array)]
struct LogEntry {
    timestamp: i64,
    message: String,
}
```

## Field Attributes

Applied to individual struct fields:

### `rename = "name"`

Override the field name in the schema:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct User {
    #[tealeaf(rename = "user_name")]
    name: String,
}
// Generates: @struct user (user_name: string)
```

### `skip`

Exclude a field from serialization/deserialization:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct User {
    name: String,
    #[tealeaf(skip)]
    internal_cache: Option<Vec<u8>>,
}
```

Skipped fields must implement `Default` for deserialization.

### `optional`

Mark a field as nullable in the schema:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct User {
    name: String,
    #[tealeaf(optional)]
    email: Option<String>,  // string?
}
```

> **Note:** Fields of type `Option<T>` are automatically detected as optional. The `#[tealeaf(optional)]` attribute is mainly useful for documentation or when using wrapper types.

### `type = "tealeaf_type"`

Override the TeaLeaf type for a field:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Event {
    #[tealeaf(type = "timestamp")]
    created_at: i64,  // Would normally be int64, but we want timestamp

    #[tealeaf(type = "uint64")]
    large_count: i64,  // Override the default signed type
}
```

Valid type names: `bool`, `int`, `int8`, `int16`, `int32`, `int64`, `uint`, `uint8`, `uint16`, `uint32`, `uint64`, `float`, `float32`, `float64`, `string`, `bytes`, `timestamp`.

### `flatten`

Inline the fields of a nested struct into the parent:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Metadata {
    created_by: String,
    version: i32,
}

#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Document {
    title: String,
    #[tealeaf(flatten)]
    meta: Metadata,
}
// Generates: @struct document (title: string, created_by: string, version: int)
// Instead of: @struct document (title: string, meta: metadata)
```

### `default`

Use `Default::default()` when deserializing a missing field:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Config {
    host: String,
    #[tealeaf(default)]
    port: i32,  // defaults to 0 if missing
}
```

### `default = "expr"`

Use a custom expression for the default value:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Config {
    host: String,
    #[tealeaf(default = "8080")]
    port: i32,
    #[tealeaf(default = "true")]
    debug: bool,
}
```

## Combining Attributes

Multiple attributes can be combined:

```rust
#[derive(ToTeaLeaf, FromTeaLeaf)]
struct Event {
    #[tealeaf(rename = "ts", type = "timestamp")]
    timestamp: i64,

    #[tealeaf(optional, rename = "msg")]
    message: Option<String>,

    #[tealeaf(skip)]
    cached_hash: u64,

    #[tealeaf(flatten)]
    metadata: EventMeta,
}
```

## Attribute Summary Table

| Attribute | Level | Description |
|-----------|-------|-------------|
| `rename = "name"` | Container or Field | Override schema/field name |
| `key = "name"` | Container | Override document key |
| `root_array` | Container | Serialize as root array element |
| `skip` | Field | Exclude from serialization |
| `optional` | Field | Mark as nullable (`T?`) |
| `type = "name"` | Field | Override TeaLeaf type |
| `flatten` | Field | Inline nested struct fields |
| `default` | Field | Use `Default::default()` |
| `default = "expr"` | Field | Use custom default expression |
