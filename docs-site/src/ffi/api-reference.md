# FFI API Reference

Complete listing of all exported FFI functions.

## Error Handling

### `tl_get_last_error`
```c
char* tl_get_last_error(void);
```
Returns the last error message, or `NULL` if no error. Caller must free with `tl_string_free`.

### `tl_clear_error`
```c
void tl_clear_error(void);
```
Clears the thread-local error state.

## Version

### `tl_version`
```c
const char* tl_version(void);
```
Returns the library version string (e.g., `"2.0.0-beta.9"`). The returned pointer is static -- do **not** free it.

## Document API

### `tl_parse`
```c
TLDocument* tl_parse(const char* text);
```
Parse a TeaLeaf text string. Returns `NULL` on failure (check `tl_get_last_error`).

### `tl_parse_file`
```c
TLDocument* tl_parse_file(const char* path);
```
Parse a TeaLeaf text file. Returns `NULL` on failure.

### `tl_document_free`
```c
void tl_document_free(TLDocument* doc);
```
Free a document. Safe to call with `NULL`.

### `tl_document_get`
```c
TLValue* tl_document_get(const TLDocument* doc, const char* key);
```
Get a value by key. Returns `NULL` if key not found or doc is `NULL`. Caller must free with `tl_value_free`.

### `tl_document_keys`
```c
char** tl_document_keys(const TLDocument* doc);
```
Get all top-level keys as a `NULL`-terminated array. Caller must free with `tl_string_array_free`.

### `tl_document_to_text`
```c
char* tl_document_to_text(const TLDocument* doc);
```
Convert document to TeaLeaf text (with schemas). Caller must free with `tl_string_free`.

### `tl_document_to_text_data_only`
```c
char* tl_document_to_text_data_only(const TLDocument* doc);
```
Convert document to TeaLeaf text (data only, no schemas). Caller must free with `tl_string_free`.

### `tl_document_to_text_with_options`
```c
char* tl_document_to_text_with_options(const TLDocument* doc, bool compact, bool compact_floats);
```
Convert document to TeaLeaf text with custom formatting options (with schemas). Set `compact` to remove insignificant whitespace. Set `compact_floats` to strip `.0` from whole-number floats (e.g., `42.0` → `42`). Caller must free with `tl_string_free`.

### `tl_document_to_text_data_only_with_options`
```c
char* tl_document_to_text_data_only_with_options(const TLDocument* doc, bool compact, bool compact_floats);
```
Convert document to TeaLeaf text with custom formatting options (data only, no schemas). Same options as `tl_document_to_text_with_options`. Caller must free with `tl_string_free`.

### `tl_document_compile`
```c
TLResult tl_document_compile(const TLDocument* doc, const char* path, bool compress);
```
Compile document to binary file. Returns a `TLResult` indicating success or failure.

## JSON API

### `tl_document_from_json`
```c
TLDocument* tl_document_from_json(const char* json);
```
Parse a JSON string into a TLDocument. Returns `NULL` on failure.

### `tl_document_to_json`
```c
char* tl_document_to_json(const TLDocument* doc);
```
Convert document to pretty-printed JSON. Caller must free with `tl_string_free`.

### `tl_document_to_json_compact`
```c
char* tl_document_to_json_compact(const TLDocument* doc);
```
Convert document to minified JSON. Caller must free with `tl_string_free`.

## Value API

### `tl_value_type`
```c
TLValueType tl_value_type(const TLValue* value);
```
Get the type of a value. Returns `TL_NULL` (0) if value is `NULL`.

### `tl_value_free`
```c
void tl_value_free(TLValue* value);
```
Free a value. Safe to call with `NULL`.

### Primitive Accessors

```c
bool    tl_value_as_bool(const TLValue* value);       // false if not bool
int64_t tl_value_as_int(const TLValue* value);        // 0 if not int
uint64_t tl_value_as_uint(const TLValue* value);      // 0 if not uint
double  tl_value_as_float(const TLValue* value);      // 0.0 if not float
char*   tl_value_as_string(const TLValue* value);     // NULL if not string; free with tl_string_free
int64_t tl_value_as_timestamp(const TLValue* value);  // 0 if not timestamp (millis only)
int16_t tl_value_as_timestamp_offset(const TLValue* value); // 0 if not timestamp (tz offset in minutes)
```

### Bytes Accessors

```c
size_t       tl_value_bytes_len(const TLValue* value);   // 0 if not bytes
const uint8_t* tl_value_bytes_data(const TLValue* value); // NULL if not bytes; pointer valid while value lives
```

### Reference/Tag Accessors

```c
char*    tl_value_ref_name(const TLValue* value);   // NULL if not ref; free with tl_string_free
char*    tl_value_tag_name(const TLValue* value);    // NULL if not tagged; free with tl_string_free
TLValue* tl_value_tag_value(const TLValue* value);   // NULL if not tagged; free with tl_value_free
```

### Array Accessors

```c
size_t   tl_value_array_len(const TLValue* value);                 // 0 if not array
TLValue* tl_value_array_get(const TLValue* value, size_t index);   // NULL if out of bounds; free with tl_value_free
```

### Object Accessors

```c
TLValue* tl_value_object_get(const TLValue* value, const char* key); // NULL if not found; free with tl_value_free
char**   tl_value_object_keys(const TLValue* value);                  // NULL-terminated; free with tl_string_array_free
```

### Map Accessors

```c
size_t   tl_value_map_len(const TLValue* value);                    // 0 if not map
TLValue* tl_value_map_get_key(const TLValue* value, size_t index);  // NULL if out of bounds; free with tl_value_free
TLValue* tl_value_map_get_value(const TLValue* value, size_t index);// NULL if out of bounds; free with tl_value_free
```

## Binary Reader API

### `tl_reader_open`
```c
TLReader* tl_reader_open(const char* path);
```
Open a binary file for reading. Returns `NULL` on failure.

### `tl_reader_open_mmap`
```c
TLReader* tl_reader_open_mmap(const char* path);
```
Open a binary file with memory-mapped I/O (zero-copy). Returns `NULL` on failure.

### `tl_reader_free`
```c
void tl_reader_free(TLReader* reader);
```
Free a reader. Safe to call with `NULL`.

### `tl_reader_get`
```c
TLValue* tl_reader_get(const TLReader* reader, const char* key);
```
Get a value by key from binary. Returns `NULL` if not found. Caller must free with `tl_value_free`.

### `tl_reader_keys`
```c
char** tl_reader_keys(const TLReader* reader);
```
Get all section keys. Returns `NULL`-terminated array. Free with `tl_string_array_free`.

## Schema API

```c
size_t tl_reader_schema_count(const TLReader* reader);
char*  tl_reader_schema_name(const TLReader* reader, size_t index);
size_t tl_reader_schema_field_count(const TLReader* reader, size_t schema_index);
char*  tl_reader_schema_field_name(const TLReader* reader, size_t schema_index, size_t field_index);
char*  tl_reader_schema_field_type(const TLReader* reader, size_t schema_index, size_t field_index);
bool   tl_reader_schema_field_nullable(const TLReader* reader, size_t schema_index, size_t field_index);
bool   tl_reader_schema_field_is_array(const TLReader* reader, size_t schema_index, size_t field_index);
```

All `char*` returns from schema functions must be freed with `tl_string_free`. Out-of-bounds indices return `NULL`/0/false.

## Memory Management

### `tl_string_free`
```c
void tl_string_free(char* s);
```
Free a string returned by any FFI function. Safe to call with `NULL`.

### `tl_string_array_free`
```c
void tl_string_array_free(char** arr);
```
Free a `NULL`-terminated string array. Frees each string and the array pointer. Safe to call with `NULL`.

### `tl_result_free`
```c
void tl_result_free(TLResult* result);
```
Free any allocated memory inside a TLResult. Safe to call with `NULL`.

## Type Enum

```c
typedef enum {
    TL_NULL      = 0,
    TL_BOOL      = 1,
    TL_INT       = 2,
    TL_UINT      = 3,
    TL_FLOAT     = 4,
    TL_STRING    = 5,
    TL_BYTES     = 6,
    TL_ARRAY     = 7,
    TL_OBJECT    = 8,
    TL_MAP       = 9,
    TL_REF       = 10,
    TL_TAGGED    = 11,
    TL_TIMESTAMP = 12,
} TLValueType;
```
