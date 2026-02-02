# Contributing Guide

Contributions to TeaLeaf are welcome. This guide covers the process.

## Ways to Contribute

- **Bug reports** -- file issues on GitHub with reproduction steps
- **Feature requests** -- open an issue describing the use case
- **Code contributions** -- submit pull requests
- **Documentation** -- fix typos, improve explanations, add examples
- **Language bindings** -- create bindings for Python, Java, Go, etc.
- **Test cases** -- add canonical test fixtures or edge case tests

## Repository

Source code: [github.com/krishjag/tealeaf](https://github.com/krishjag/tealeaf)

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run all tests (see below)
5. Submit a pull request against `main`

## Code Style

### Rust

- Follow standard `rustfmt` formatting
- Use `clippy` for lint checks
- Document public APIs with `///` doc comments

### C# (.NET)

- Follow standard C# naming conventions
- Use XML doc comments for public APIs
- Target .NET 8.0

## Running Tests

### Before Submitting

```bash
# Rust: all tests + clippy
cargo test --workspace
cargo clippy --workspace

# .NET tests
cd bindings/dotnet
dotnet test
```

### Adding Tests

- **Canonical fixtures:** Add `.tl` file to `canonical/samples/`, expected JSON to `canonical/expected/`
- **Rust tests:** Add to `tealeaf-core/tests/` or inline `#[test]` functions
- **.NET tests:** Add to the appropriate test project

## Areas of Interest

### New Language Bindings

The FFI layer exposes a C-compatible API that can be used from any language. See the [FFI Overview](../ffi/overview.md) and [Building from Source](../ffi/building.md) for getting started.

Desired bindings:
- Python (via `ctypes` or `cffi`)
- Java/Kotlin (via JNI or JNA)
- Go (via cgo)
- JavaScript/TypeScript (via WASM or N-API)

### Format Improvements

- Union support in binary encoding
- Bytes literal syntax in text format
- Streaming/append-only mode

### Tooling

- Editor plugins (VS Code syntax highlighting for `.tl`)
- Schema validation tooling
- Web-based playground

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
