# Installation

## Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/krishjag/tealeaf/releases/latest).

| Platform | Architecture | Download |
|----------|-------------|----------|
| Windows | x64 | [tealeaf-windows-x64.zip](https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-windows-x64.zip) |
| Windows | ARM64 | [tealeaf-windows-arm64.zip](https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-windows-arm64.zip) |
| Linux | x64 (glibc) | [tealeaf-linux-x64.tar.gz](https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-linux-x64.tar.gz) |
| Linux | ARM64 (glibc) | [tealeaf-linux-arm64.tar.gz](https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-linux-arm64.tar.gz) |
| Linux | x64 (musl) | [tealeaf-linux-musl-x64.tar.gz](https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-linux-musl-x64.tar.gz) |
| macOS | x64 (Intel) | [tealeaf-macos-x64.tar.gz](https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-macos-x64.tar.gz) |
| macOS | ARM64 (Apple Silicon) | [tealeaf-macos-arm64.tar.gz](https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-macos-arm64.tar.gz) |

## Quick Install

### Windows (PowerShell)

```powershell
# Download and extract to current directory
Invoke-WebRequest -Uri "https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-windows-x64.zip" -OutFile tealeaf.zip
Expand-Archive tealeaf.zip -DestinationPath .

# Optional: add to PATH
$env:PATH += ";$PWD"
```

### Linux / macOS

```bash
# Download and extract (replace with your platform)
curl -LO https://github.com/krishjag/tealeaf/releases/latest/download/tealeaf-linux-x64.tar.gz
tar -xzf tealeaf-linux-x64.tar.gz

# Optional: move to PATH
sudo mv tealeaf /usr/local/bin/
```

## Build from Source

Requires the [Rust toolchain](https://rustup.rs) (1.70+).

```bash
git clone https://github.com/krishjag/tealeaf.git
cd tealeaf
cargo build --release --package tealeaf-core
```

The binary will be at `target/release/tealeaf` (or `tealeaf.exe` on Windows).

## Verify Installation

```bash
tealeaf --version
# tealeaf 2.0.0-beta.14

tealeaf help
```

## Rust Crate

Add `tealeaf-core` to your `Cargo.toml`:

```toml
[dependencies]
tealeaf-core = { version = "2.0.0-beta.14", features = ["derive"] }
```

The `derive` feature enables `#[derive(ToTeaLeaf, FromTeaLeaf)]` macros.

## .NET NuGet Package

```bash
dotnet add package TeaLeaf
```

The NuGet package includes everything needed:
- `TeaLeaf.Annotations` -- `[TeaLeaf]`, `[TLSkip]`, and other attributes
- `TeaLeaf.Generators` -- C# incremental source generator (bundled as an analyzer)
- Native libraries for all supported platforms (Windows, Linux, macOS -- x64 and ARM64)

No additional packages required. `[TeaLeaf]` classes get compile-time serialization methods automatically.

> **Note:** The .NET package requires .NET 8.0 or later. The source generator requires a C# compiler with incremental generator support.
