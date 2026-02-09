# Building from Source

How to build the TeaLeaf FFI library from source.

## Prerequisites

- [Rust toolchain](https://rustup.rs) (1.70+)
- A C compiler (for cbindgen header generation)

## Build

```bash
git clone https://github.com/krishjag/tealeaf.git
cd tealeaf
cargo build --release --package tealeaf-ffi
```

### Output Files

| Platform | Dynamic Library | Static Library |
|----------|----------------|----------------|
| Windows | `target/release/tealeaf_ffi.dll` | `target/release/tealeaf_ffi.lib` |
| Linux | `target/release/libtealeaf_ffi.so` | `target/release/libtealeaf_ffi.a` |
| macOS | `target/release/libtealeaf_ffi.dylib` | `target/release/libtealeaf_ffi.a` |

### C Header

The build generates a C header via `cbindgen` (configured in `tealeaf-ffi/cbindgen.toml`):

```bash
# Header is generated during build
# Location: target/tealeaf_ffi.h (or as configured)
```

## Cross-Compilation

### Linux ARM64

```bash
# Install cross-compilation tools
sudo apt install gcc-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu

# Build
cargo build --release --package tealeaf-ffi --target aarch64-unknown-linux-gnu
```

### Windows ARM64

```bash
rustup target add aarch64-pc-windows-msvc
cargo build --release --package tealeaf-ffi --target aarch64-pc-windows-msvc
```

### macOS (from any platform via cross)

```bash
# Using cross (https://github.com/cross-rs/cross)
cargo install cross
cross build --release --package tealeaf-ffi --target aarch64-apple-darwin
cross build --release --package tealeaf-ffi --target x86_64-apple-darwin
```

## Linking

### Dynamic Linking

```bash
# GCC/Clang
gcc -o myapp myapp.c -L/path/to/lib -ltealeaf_ffi

# MSVC
cl myapp.c /link tealeaf_ffi.lib
```

At runtime, ensure the dynamic library is in the library search path.

### Static Linking

```bash
# GCC/Clang (Linux)
gcc -o myapp myapp.c /path/to/libtealeaf_ffi.a -lpthread -ldl -lm

# macOS
gcc -o myapp myapp.c /path/to/libtealeaf_ffi.a -framework Security -lpthread
```

Static linking eliminates the runtime dependency but produces a larger binary.

## Dependencies

The FFI crate has minimal dependencies:

```toml
[dependencies]
tealeaf-core = { workspace = true }

[build-dependencies]
cbindgen = "0.27"
```

The resulting library links against:
- **Linux:** `libpthread`, `libdl`, `libm`
- **macOS:** `Security.framework`, `libpthread`
- **Windows:** standard Windows system libraries

## Writing New Language Bindings

To create bindings for a new language:

1. **Generate or write FFI declarations** matching the C header
2. **Load the dynamic library** (or link statically)
3. **Wrap opaque pointers** in your language's resource management (destructors, `Dispose`, `__del__`, etc.)
4. **Map the error model** -- check for NULL returns and call `tl_get_last_error`
5. **Handle string ownership** -- copy strings to your language's string type, then free the C string

### Example: Python (ctypes)

```python
import ctypes

lib = ctypes.CDLL("libtealeaf_ffi.so")

# Define function signatures
lib.tl_parse.restype = ctypes.c_void_p
lib.tl_parse.argtypes = [ctypes.c_char_p]

lib.tl_document_get.restype = ctypes.c_void_p
lib.tl_document_get.argtypes = [ctypes.c_void_p, ctypes.c_char_p]

lib.tl_value_as_string.restype = ctypes.c_char_p
lib.tl_value_as_string.argtypes = [ctypes.c_void_p]

# Use it
doc = lib.tl_parse(b"name: alice")
val = lib.tl_document_get(doc, b"name")
name = lib.tl_value_as_string(val)
print(name.decode())  # "alice"

lib.tl_value_free(val)
lib.tl_document_free(doc)
```

## Testing

```bash
# Run FFI tests
cargo test --package tealeaf-ffi

# Run all workspace tests
cargo test --workspace
```
