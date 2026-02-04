# Contributing Guide

Contributions to TeaLeaf are welcome. The full contributing guide lives in the repository root:

**[CONTRIBUTING.md](https://github.com/krishjag/tealeaf/blob/main/CONTRIBUTING.md)**

That document covers project architecture, build instructions, testing, the canonical test suite, version management, PR process, and areas of interest for contributors.

This page highlights the key points. See the [Development Setup](development.md) page for environment setup details.

## Ways to Contribute

- **Bug reports** -- file issues on GitHub with reproduction steps
- **Feature requests** -- open an issue describing the use case
- **Code contributions** -- submit pull requests
- **Documentation** -- fix typos, improve explanations, add examples
- **Language bindings** -- create bindings for Python, Java, Go, etc.
- **Test cases** -- add canonical test fixtures or edge case tests

## Repository

Source code: [github.com/krishjag/tealeaf](https://github.com/krishjag/tealeaf)

## Pull Request Checklist

1. Fork the repository and create a feature branch from `main`
2. Make your changes
3. Run tests and lints:
   ```bash
   cargo test --workspace
   cargo clippy --workspace
   cargo fmt --check
   ```
4. If you modified .NET bindings: `cd bindings/dotnet && dotnet test`
5. Submit a pull request against `main`

CI runs on Linux, macOS, and Windows automatically. Version consistency is validated on every PR.

## Code Style

### Rust

- Standard `rustfmt` formatting (no custom config)
- Standard `clippy` lints (no custom config)
- Document public APIs with `///` doc comments
- Edition 2021

### C# (.NET)

- Standard C# naming conventions
- XML doc comments for public APIs
- Target frameworks: net6.0, net8.0, net10.0, netstandard2.0

## Areas of Interest

### New Language Bindings

The FFI layer exposes a C-compatible API that can be used from any language. See the [FFI Overview](../ffi/overview.md) for getting started.

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
