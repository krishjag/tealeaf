# Contributing to TeaLeaf

Contributions are welcome. This guide covers what you need to get started.

## Quick Start

```bash
git clone https://github.com/krishjag/tealeaf.git
cd tealeaf
cargo build --workspace
cargo test --workspace
```

If you're working on .NET bindings, also build the native library first:

```bash
cargo build --package tealeaf-ffi
cd bindings/dotnet && dotnet build && dotnet test
```

## Project Architecture

TeaLeaf is a Cargo workspace with four crates and a .NET binding layer:

```
tealeaf/
├── tealeaf-core/          Core library + CLI binary
│   ├── src/lib.rs         Lexer, parser, type system
│   ├── src/reader.rs      Binary format decoder
│   ├── src/writer.rs      Binary format encoder
│   ├── src/convert.rs     JSON conversion
│   ├── src/builder.rs     Programmatic document builder
│   ├── src/main.rs        CLI entry point
│   ├── tests/             Integration tests (canonical, CLI, derive)
│   ├── benches/           Criterion benchmarks
│   └── examples/          Size report, usage examples
│
├── tealeaf-derive/        Proc-macro crate for derive-based DTO conversion
│   └── src/lib.rs         #[derive(TeaLeafSerialize, TeaLeafDeserialize)]
│
├── tealeaf-ffi/           C FFI layer for language bindings
│   ├── src/lib.rs         Exported C functions
│   └── cbindgen.toml      Auto-generates tealeaf.h during build
│
├── accuracy-benchmark/    LLM accuracy benchmark suite
│   ├── src/providers/     Anthropic and OpenAI API clients
│   ├── src/runner/        Executor, rate limiter
│   ├── src/tasks/         Task definitions and loader
│   ├── src/analysis/      Scoring and comparison engine
│   └── config/            TOML model configuration
│
├── bindings/dotnet/       .NET bindings (NuGet package)
│   ├── TeaLeaf/           Main library (P/Invoke over FFI)
│   ├── TeaLeaf.Annotations/   Serialization attributes
│   ├── TeaLeaf.Generators/    Roslyn source generators
│   └── TeaLeaf.Tests/         Test suite
│
├── canonical/             Shared test fixtures (14 sample files)
│   ├── samples/           Input .tl files
│   ├── expected/          Expected JSON outputs
│   ├── binary/            Pre-compiled .tlbx files
│   └── errors/            Invalid inputs + expected error messages
│
├── spec/                  Format specification (TEALEAF_SPEC.md)
├── docs-site/             Documentation site (mdBook)
├── examples/              Example files (retail orders, etc.)
├── test-vectors/          Additional test data
└── scripts/               Version sync, coverage collection
```

### Crate Dependency Graph

```
tealeaf-derive (proc-macro, no internal deps)
       │
       ▼
tealeaf-core (depends on tealeaf-derive)
       │
       ├──▶ tealeaf-ffi (depends on tealeaf-core)
       │         │
       │         └──▶ bindings/dotnet (P/Invoke over FFI .dll/.so/.dylib)
       │
       └──▶ accuracy-benchmark (depends on tealeaf-core)
```

Changes to `tealeaf-core` affect everything downstream. Changes to `tealeaf-derive` affect `tealeaf-core` (and therefore everything). Changes to `tealeaf-ffi` affect .NET bindings but not the Rust library or CLI.

## Prerequisites

| Tool | Version | Required For |
|------|---------|--------------|
| [Rust](https://rustup.rs) | 1.70+ | All Rust crates |
| [.NET SDK](https://dotnet.microsoft.com/download) | 8.0+ | .NET bindings |
| [Git](https://git-scm.com) | Any | Version control |

Optional:

| Tool | Required For |
|------|--------------|
| [mdBook](https://rust-lang.github.io/mdBook/) | Building documentation site |
| [Protobuf compiler](https://grpc.io/docs/protoc-installation/) | Benchmark protobuf comparisons |
| [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) | Code coverage |

## Building

```bash
# Full workspace (debug)
cargo build --workspace

# Release build with LTO
cargo build --workspace --release

# Single crate
cargo build --package tealeaf-core
cargo build --package tealeaf-ffi
cargo build --package accuracy-benchmark

# .NET (requires tealeaf-ffi built first)
cargo build --package tealeaf-ffi
cd bindings/dotnet
dotnet build
```

## Testing

### Rust Tests

```bash
# All workspace tests
cargo test --workspace

# Specific crate
cargo test --package tealeaf-core
cargo test --package tealeaf-derive
cargo test --package tealeaf-ffi

# Canonical test suite (52 tests: 42 success + 10 error)
cargo test --package tealeaf-core --test canonical

# CLI integration tests
cargo test --package tealeaf-core --test cli_integration

# Derive macro tests
cargo test --package tealeaf-core --test derive

# With output visible
cargo test --workspace -- --nocapture
```

### .NET Tests

```bash
cd bindings/dotnet
dotnet test

# Specific test filter
dotnet test --filter "Canonical"
```

### Linting

```bash
cargo clippy --workspace
cargo fmt --check
```

### Benchmarks

```bash
# Criterion benchmarks
cargo bench --package tealeaf-core

# Size comparison report
cargo run --package tealeaf-core --example size_report
```

## Canonical Test Suite

The canonical suite in `canonical/` is the primary correctness gate. Every parser change, binary format change, or JSON conversion change must pass these tests.

### How It Works

Each test case has three representations that must agree:

```
canonical/samples/foo.tl          Text input
canonical/expected/foo.json       Expected JSON output
canonical/binary/foo.tlbx         Pre-compiled binary
```

The test runner validates three paths:
1. **Text -> JSON**: Parse `.tl`, export JSON, compare with `expected/`
2. **Binary -> JSON**: Read `.tlbx`, export JSON, compare with `expected/`
3. **Round-trip**: Parse `.tl` -> compile to `.tlbx` -> read back -> export JSON

### Adding a Test Case

1. Create `canonical/samples/your_test.tl`
2. Generate expected outputs:
   ```bash
   tealeaf to-json canonical/samples/your_test.tl -o canonical/expected/your_test.json
   tealeaf compile canonical/samples/your_test.tl -o canonical/binary/your_test.tlbx
   ```
3. Add test functions to `tealeaf-core/tests/canonical.rs`
4. Run: `cargo test --package tealeaf-core --test canonical`
5. Commit all three files together

### Adding an Error Test Case

Error tests live in `canonical/errors/` with expected messages in `expected_errors.json`.

1. Create an invalid input file in `canonical/errors/`
2. Add the expected error message to `canonical/errors/expected_errors.json`
3. Run: `cargo test -p tealeaf-core error_`

Error messages are considered a public contract -- changes to error text require a version bump.

## Version Management

The single source of truth for the project version is `release.json` in the repository root. All `Cargo.toml` files, `.csproj` files, and other version references must match.

```bash
# Check if all versions are in sync
./scripts/validate-version.sh        # Unix
./scripts/validate-version.ps1       # Windows

# Sync versions from release.json to all files
./scripts/sync-version.sh            # Unix
./scripts/sync-version.ps1           # Windows
```

CI runs `validate-version.sh` on every push and PR. If your PR changes the version, run `sync-version` locally and commit the result.

## Development Workflows

### Modifying the Parser or Type System

The lexer and parser live in `tealeaf-core/src/lib.rs`. The type system (`Value`, `TeaLeaf`, schema types) is also defined there.

1. Make your changes in `tealeaf-core/src/lib.rs`
2. Run canonical tests: `cargo test --package tealeaf-core --test canonical`
3. Run full test suite: `cargo test --package tealeaf-core`
4. Add new canonical test cases if the change adds syntax or types

### Modifying the Binary Format

The encoder is in `tealeaf-core/src/writer.rs`, the decoder in `tealeaf-core/src/reader.rs`. Binary format changes are breaking changes.

1. Edit `writer.rs` and/or `reader.rs`
2. Run canonical round-trip tests: `cargo test --package tealeaf-core --test canonical`
3. If the on-disk format changed, regenerate all `.tlbx` fixtures (see canonical suite section above)
4. Update `spec/TEALEAF_SPEC.md` if the format changed

### Modifying Derive Macros

1. Edit `tealeaf-derive/src/lib.rs`
2. Run: `cargo test --package tealeaf-core --test derive`
3. Add test cases for new functionality

### Modifying the FFI Layer

1. Edit `tealeaf-ffi/src/lib.rs`
2. Run: `cargo test --package tealeaf-ffi`
3. The C header (`tealeaf.h`) is auto-regenerated by cbindgen during build
4. If you added or changed exported functions, update the .NET P/Invoke declarations in `bindings/dotnet/TeaLeaf/Internal/`

### Modifying .NET Bindings

1. Build the native library: `cargo build --package tealeaf-ffi`
2. Edit files in `bindings/dotnet/`
3. Build: `cd bindings/dotnet && dotnet build`
4. Test: `dotnet test`

### Working on the Accuracy Benchmark

The benchmark suite tests TeaLeaf vs JSON format performance across LLM providers (Anthropic, OpenAI). It requires API keys set as environment variables.

1. Build: `cargo build --package accuracy-benchmark`
2. List tasks: `cargo run --package accuracy-benchmark -- list-tasks`
3. Dump prompts without making API calls: `cargo run --package accuracy-benchmark -- dump-prompts`
4. Run with API calls: `cargo run --package accuracy-benchmark -- run --compare-formats --providers anthropic`

Configuration is in `accuracy-benchmark/config/models.toml`. Rate limits, model selection, and thinking parameters are all configurable there.

## Pull Request Process

1. Fork the repository and create a feature branch from `main`
2. Make your changes
3. Run tests:
   ```bash
   cargo test --workspace
   cargo clippy --workspace
   cargo fmt --check
   ```
4. If you modified .NET bindings:
   ```bash
   cd bindings/dotnet && dotnet test
   ```
5. Submit a PR against `main`

### What CI Checks

| Workflow | What it does |
|----------|-------------|
| `rust-cli.yml` | Builds and tests on Linux, macOS, Windows; builds release binaries for 7 targets |
| `dotnet-package.yml` | Builds native FFI libraries for 6 platforms; packages and tests NuGet |
| `accuracy-benchmark.yml` | Builds and tests the benchmark suite on all platforms |
| `docs.yml` | Builds mdBook documentation and Rust API docs |

All workflows run on push to `main`/`develop` and on pull requests. Version validation runs on every PR.

### PR Guidelines

- Keep PRs focused on a single concern
- If a change touches the format specification, update `spec/TEALEAF_SPEC.md`
- If a change adds public API surface, add doc comments (`///` in Rust, XML `<summary>` in C#)
- Binary format changes are breaking changes and require discussion first
- Error message changes should be noted since they are part of the public contract

## Code Style

### Rust

- Standard `rustfmt` formatting (no custom config)
- Standard `clippy` lints (no custom config)
- Document public APIs with `///` doc comments
- Use `thiserror` for error types
- Edition 2021

### C# (.NET)

- Standard C# naming conventions
- XML doc comments for public APIs
- Target frameworks: net6.0, net8.0, net10.0, netstandard2.0

### General

- No tabs; use spaces
- Keep lines reasonable (no hard limit, but ~100 chars is a good target)
- Commit messages: imperative mood, concise first line, details in body if needed

## Cross-Platform Notes

TeaLeaf builds and tests on Linux, macOS, and Windows. A few things to watch for:

- **Line endings**: CI sets `git config core.autocrlf false`. Don't rely on CRLF.
- **Path separators**: Use `std::path::Path` and `PathBuf` in Rust, not hardcoded `/` or `\`.
- **CI matrix**: If your change is platform-sensitive, it will be tested on all three OSes automatically.
- **FFI naming**: The native library is `tealeaf_ffi.dll` (Windows), `libtealeaf_ffi.so` (Linux), `libtealeaf_ffi.dylib` (macOS). The .NET package includes all variants organized by RID (e.g., `runtimes/win-x64/native/`).

## Documentation

### Building the Documentation Site

```bash
cargo install mdbook
cd docs-site
mdbook serve --open
```

### Rust API Docs

```bash
cargo doc --workspace --no-deps --open
```

### Documentation Structure

The docs site is an mdBook project in `docs-site/`. Key sections:

- `src/introduction.md` -- Overview and primary use case
- `src/cli/` -- CLI command reference
- `src/dotnet/` -- .NET binding documentation
- `src/ffi/` -- FFI API reference
- `src/guides/` -- Usage guides (LLM context, binary format, etc.)
- `src/contributing/` -- Contributing and development guides
- `src/appendix/` -- Changelog, type reference, comparison matrix

## Areas of Interest

### New Language Bindings

The FFI layer (`tealeaf-ffi`) exposes a C-compatible API. Any language that can call C functions can build bindings. Desired bindings:

- **Python** via `ctypes` or `cffi`
- **Java/Kotlin** via JNI or JNA
- **Go** via cgo
- **JavaScript/TypeScript** via WASM or N-API

See `tealeaf-ffi/src/lib.rs` for the exported API and `bindings/dotnet/` as a reference implementation.

### Format Improvements

- Union support in binary encoding
- Bytes literal syntax in text format
- Streaming/append-only mode

### Tooling

- VS Code syntax highlighting extension for `.tl` files
- Schema validation tooling
- Web-based playground

## Debugging

### Rust

```bash
# Debug logging
RUST_LOG=debug cargo run --package tealeaf-core -- info test.tl

# Backtraces on panic
RUST_BACKTRACE=1 cargo test --package tealeaf-core
```

### .NET

Use Visual Studio or VS Code with the C# extension. For native library issues, attach a native debugger to the .NET test process.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
