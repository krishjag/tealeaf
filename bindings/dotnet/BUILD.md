# Building TeaLeaf .NET Bindings

This document describes how to build the TeaLeaf .NET library with its native dependencies.

## Prerequisites

- **Rust toolchain** - Install from [rustup.rs](https://rustup.rs)
- **.NET SDK 8.0+** - Install from [dot.net](https://dot.net)

## Quick Start

### Windows (PowerShell)

```powershell
# Build for current platform
.\build.ps1

# Build and create NuGet package
.\build.ps1 -Pack

# Build with tests
.\build.ps1 -Test -Pack
```

### Linux/macOS (Bash)

```bash
# Build for current platform
./build.sh

# Build and create NuGet package
./build.sh -p

# Build with tests
./build.sh -t -p
```

## Build Scripts

### PowerShell (build.ps1)

| Parameter | Description | Default |
|-----------|-------------|---------|
| `-Configuration` | Debug or Release | Release |
| `-TargetRids` | RIDs to build (comma-separated, or `all`) | Native platform |
| `-Test` | Run tests after building | false |
| `-Pack` | Create NuGet package | false |
| `-SkipRust` | Skip Rust compilation | false |

**Examples:**

```powershell
# Build all platforms (requires cross-compilation setup)
.\build.ps1 -TargetRids all -Pack

# Build specific platforms
.\build.ps1 -TargetRids win-x64,win-arm64

# Debug build
.\build.ps1 -Configuration Debug -Test
```

### Bash (build.sh)

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --configuration` | Debug or Release | Release |
| `-r, --rids` | RIDs to build (comma-separated, or `all`) | Native platform |
| `-t, --test` | Run tests after building | false |
| `-p, --pack` | Create NuGet package | false |
| `-s, --skip-rust` | Skip Rust compilation | false |
| `-h, --help` | Show help | - |

**Examples:**

```bash
# Build all platforms
./build.sh -r all -p

# Build specific platforms
./build.sh -r linux-x64,linux-arm64

# Debug build with tests
./build.sh -c Debug -t
```

## Supported Platforms

| RID | Rust Target | Notes |
|-----|-------------|-------|
| `win-x64` | x86_64-pc-windows-msvc | Windows 64-bit |
| `win-arm64` | aarch64-pc-windows-msvc | Windows ARM64 |
| `linux-x64` | x86_64-unknown-linux-gnu | Linux 64-bit (glibc) |
| `linux-arm64` | aarch64-unknown-linux-gnu | Linux ARM64 (glibc) |
| `osx-x64` | x86_64-apple-darwin | macOS Intel |
| `osx-arm64` | aarch64-apple-darwin | macOS Apple Silicon |

## Cross-Compilation

Cross-compiling for other platforms requires additional setup:

### Windows Cross-Compilation

Windows can only natively compile for Windows targets. For other platforms, use the GitHub Actions workflow.

### Linux Cross-Compilation

```bash
# Install ARM64 cross-compiler
sudo apt-get install gcc-aarch64-linux-gnu

# Install Rust targets
rustup target add aarch64-unknown-linux-gnu
```

### macOS Cross-Compilation

macOS can compile for both x64 and ARM64:

```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

## Project Structure

```
bindings/dotnet/
├── TeaLeaf/                # Main library project
│   ├── TeaLeaf.csproj
│   ├── NativeLibrary.cs    # P/Invoke declarations
│   └── runtimes/           # Native libraries (populated by build)
│       ├── win-x64/native/
│       ├── linux-x64/native/
│       └── ...
├── TeaLeaf.Tests/          # Test project
├── build.ps1               # Windows build script
├── build.sh                # Unix build script
└── artifacts/              # Output directory for packages
```

## GitHub Actions

The repository includes a GitHub Actions workflow (`.github/workflows/dotnet-package.yml`) that:

1. Builds native libraries on Windows, Linux, and macOS runners
2. Combines all libraries into a single NuGet package
3. Publishes to NuGet.org on release (requires `NUGET_API_KEY` secret)

### Manual Workflow Trigger

You can manually trigger the workflow with the "Publish to NuGet" option from the Actions tab.

## NuGet Package Structure

The final NuGet package includes native libraries for all platforms:

```
TeaLeaf.2.0.0-beta.7.nupkg
├── lib/
│   ├── net8.0/
│   │   └── TeaLeaf.dll
│   └── net10.0/
│       └── TeaLeaf.dll
└── runtimes/
    ├── win-x64/native/tealeaf_ffi.dll
    ├── win-arm64/native/tealeaf_ffi.dll
    ├── linux-x64/native/libtealeaf_ffi.so
    ├── linux-arm64/native/libtealeaf_ffi.so
    ├── osx-x64/native/libtealeaf_ffi.dylib
    └── osx-arm64/native/libtealeaf_ffi.dylib
```

## Troubleshooting

### Native library not found at runtime

Ensure the native library is in the correct location. The .NET runtime looks for native libraries in:

1. Application directory
2. `runtimes/{rid}/native/` subdirectory
3. System library paths

### Cross-compilation fails

Install the appropriate cross-compilation toolchain for your target platform. See the Cross-Compilation section above.

### Tests fail to find native library

The build scripts copy the native library to the test output directory. If tests still fail, manually copy the library:

```bash
cp TeaLeaf/runtimes/{your-rid}/native/libtealeaf_ffi.* TeaLeaf.Tests/bin/Release/net8.0/
```
