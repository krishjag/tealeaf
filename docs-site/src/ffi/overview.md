# FFI Reference: Overview

The `tealeaf-ffi` crate exposes a C-compatible API for integrating TeaLeaf into any language that supports C FFI (Foreign Function Interface).

## Architecture

```
┌──────────────────────┐
│  Host Language       │
│  (.NET, Python, etc.)│
├──────────────────────┤
│  FFI Bindings        │
│  (P/Invoke, ctypes)  │
├──────────────────────┤
│  tealeaf_ffi         │  ← C ABI library
│  (cdylib + staticlib)│
├──────────────────────┤
│  tealeaf-core        │  ← Rust core library
└──────────────────────┘
```

The FFI layer provides:
- **Document parsing** -- parse text, files, and JSON
- **Value access** -- type-safe accessors for all value types
- **Binary reader** -- read `.tlbx` files with optional memory mapping
- **Schema introspection** -- query schema structure at runtime
- **JSON conversion** -- to/from JSON
- **Binary compilation** -- compile documents to `.tlbx`
- **Error handling** -- thread-local last-error pattern
- **Memory management** -- explicit free functions for all allocated resources

## Output Libraries

The crate builds both dynamic and static libraries:

| Platform | Dynamic Library | Static Library |
|----------|----------------|----------------|
| Windows | `tealeaf_ffi.dll` | `tealeaf_ffi.lib` |
| Linux | `libtealeaf_ffi.so` | `libtealeaf_ffi.a` |
| macOS | `libtealeaf_ffi.dylib` | `libtealeaf_ffi.a` |

## C Header

The build generates a C header via `cbindgen`:

```c
#include "tealeaf_ffi.h"

// Parse a document
TLDocument* doc = tl_parse("name: alice\nage: 30");
if (!doc) {
    char* err = tl_get_last_error();
    fprintf(stderr, "Error: %s\n", err);
    tl_string_free(err);
    return 1;
}

// Access a value
TLValue* val = tl_document_get(doc, "name");
if (val && tl_value_type(val) == TL_STRING) {
    char* name = tl_value_as_string(val);
    printf("Name: %s\n", name);
    tl_string_free(name);
}

tl_value_free(val);
tl_document_free(doc);
```

## Opaque Types

The FFI uses opaque pointer types:

| Type | Description |
|------|-------------|
| `TLDocument*` | Parsed document handle |
| `TLValue*` | Value handle (any type) |
| `TLReader*` | Binary file reader handle |

All handles must be freed with their corresponding `_free` function.

## Error Model

TeaLeaf FFI uses the thread-local last-error pattern:

1. Functions that can fail return `NULL` (pointers) or a result struct
2. On failure, the error message is stored in thread-local storage
3. Call `tl_get_last_error()` to retrieve it
4. Call `tl_clear_error()` to clear it

```c
TLDocument* doc = tl_parse("invalid {");
if (!doc) {
    char* err = tl_get_last_error();
    // err contains the parse error message
    tl_string_free(err);
}
```

## Null Safety

All FFI functions that accept pointers are null-safe:
- Passing `NULL` returns a safe default (0, false, NULL) rather than crashing
- This makes it safe to chain calls without checking each one

## Next Steps

- [API Reference](./api-reference.md) -- complete function listing
- [Memory Management](./memory-management.md) -- ownership and freeing rules
- [Building from Source](./building.md) -- compilation instructions
