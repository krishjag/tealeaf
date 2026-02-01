# Memory Management

The FFI layer uses explicit manual memory management. Understanding ownership rules is critical for writing correct bindings.

## Ownership Rules

### Rule 1: Caller Owns Returned Pointers

Every function that returns a heap-allocated pointer transfers ownership to the caller. The caller **must** free it with the appropriate function:

| Return Type | Free Function |
|-------------|---------------|
| `TLDocument*` | `tl_document_free()` |
| `TLValue*` | `tl_value_free()` |
| `TLReader*` | `tl_reader_free()` |
| `char*` | `tl_string_free()` |
| `char**` | `tl_string_array_free()` |
| `TLResult` | `tl_result_free()` |

### Rule 2: Borrowed Pointers Are Read-Only

Functions that take `const T*` parameters borrow the pointer. The FFI layer does **not** take ownership or free inputs:

```c
// doc is borrowed — you still own it and must free it later
TLValue* val = tl_document_get(doc, "key");
// ... use val ...
tl_value_free(val);  // free the returned value
tl_document_free(doc);  // free the document separately
```

### Rule 3: Null Is Always Safe

Every free function and every accessor accepts `NULL` safely:

```c
tl_document_free(NULL);  // no-op
tl_value_free(NULL);     // no-op
tl_string_free(NULL);    // no-op

TLValue* val = tl_document_get(NULL, "key");  // returns NULL
bool b = tl_value_as_bool(NULL);              // returns false
```

## Common Patterns

### Parse → Use → Free

```c
TLDocument* doc = tl_parse("name: alice");
if (doc) {
    TLValue* name = tl_document_get(doc, "name");
    if (name) {
        char* str = tl_value_as_string(name);
        if (str) {
            printf("%s\n", str);
            tl_string_free(str);
        }
        tl_value_free(name);
    }
    tl_document_free(doc);
}
```

### Iterating Arrays

```c
TLValue* arr = tl_document_get(doc, "items");
size_t len = tl_value_array_len(arr);

for (size_t i = 0; i < len; i++) {
    TLValue* elem = tl_value_array_get(arr, i);
    // use elem...
    tl_value_free(elem);  // free each element
}

tl_value_free(arr);  // free the array value
```

### Iterating Object Keys

```c
TLValue* obj = tl_document_get(doc, "config");
char** keys = tl_value_object_keys(obj);

if (keys) {
    for (int i = 0; keys[i] != NULL; i++) {
        TLValue* val = tl_value_object_get(obj, keys[i]);
        // use val...
        tl_value_free(val);
    }
    tl_string_array_free(keys);  // frees all strings AND the array
}

tl_value_free(obj);
```

### Iterating Maps

```c
TLValue* map = tl_document_get(doc, "headers");
size_t len = tl_value_map_len(map);

for (size_t i = 0; i < len; i++) {
    TLValue* key = tl_value_map_get_key(map, i);
    TLValue* val = tl_value_map_get_value(map, i);

    char* k = tl_value_as_string(key);
    char* v = tl_value_as_string(val);
    printf("%s: %s\n", k, v);

    tl_string_free(k);
    tl_string_free(v);
    tl_value_free(key);
    tl_value_free(val);
}

tl_value_free(map);
```

### String Arrays

```c
char** keys = tl_document_keys(doc);
if (keys) {
    for (int i = 0; keys[i] != NULL; i++) {
        printf("Key: %s\n", keys[i]);
    }
    tl_string_array_free(keys);  // ONE call frees everything
}
```

## Bytes Data

The `tl_value_bytes_data` function returns a **borrowed** pointer valid only while the value lives:

```c
TLValue* val = tl_document_get(doc, "data");
size_t len = tl_value_bytes_len(val);
const uint8_t* data = tl_value_bytes_data(val);

// Copy if you need the data after freeing the value
uint8_t* copy = malloc(len);
memcpy(copy, data, len);

tl_value_free(val);  // data pointer is now invalid
// copy is still valid
```

## Error Strings

Error strings are owned by the caller:

```c
char* err = tl_get_last_error();
if (err) {
    fprintf(stderr, "Error: %s\n", err);
    tl_string_free(err);  // must free
}
```

## Common Mistakes

| Mistake | Consequence | Fix |
|---------|------------|-----|
| Not freeing returned pointers | Memory leak | Always pair creation with `_free` |
| Using pointer after free | Use-after-free / crash | Set pointer to NULL after free |
| Freeing borrowed `bytes_data` pointer | Double-free / crash | Only free with `tl_value_free` on the value |
| Calling wrong free function | Undefined behavior | Match the free to the allocation type |
| Freeing strings from string_array individually | Double-free | Use `tl_string_array_free` once |
