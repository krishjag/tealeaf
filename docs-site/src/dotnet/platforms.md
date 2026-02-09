# Platform Support

The TeaLeaf NuGet package includes pre-built native libraries for all major platforms.

## Supported Platforms

| OS | Architecture | Native Library | Status |
|----|-------------|----------------|--------|
| Windows | x64 | `tealeaf_ffi.dll` | Supported |
| Windows | ARM64 | `tealeaf_ffi.dll` | Supported |
| Linux | x64 (glibc) | `libtealeaf_ffi.so` | Supported |
| Linux | ARM64 (glibc) | `libtealeaf_ffi.so` | Supported |
| macOS | x64 (Intel) | `libtealeaf_ffi.dylib` | Supported |
| macOS | ARM64 (Apple Silicon) | `libtealeaf_ffi.dylib` | Supported |

## .NET Requirements

- **.NET 8.0** or later
- C# compiler with incremental source generator support (for the source generator)

## NuGet Package Structure

The NuGet package bundles native libraries for all platforms using the `runtimes` folder convention:

```
TeaLeaf.nupkg
├── lib/net8.0/
│   ├── TeaLeaf.dll
│   ├── TeaLeaf.Annotations.dll
│   └── TeaLeaf.Generators.dll
└── runtimes/
    ├── win-x64/native/tealeaf_ffi.dll
    ├── win-arm64/native/tealeaf_ffi.dll
    ├── linux-x64/native/libtealeaf_ffi.so
    ├── linux-arm64/native/libtealeaf_ffi.so
    ├── osx-x64/native/libtealeaf_ffi.dylib
    └── osx-arm64/native/libtealeaf_ffi.dylib
```

The .NET runtime automatically selects the correct native library based on the host platform.

## Native Library Loading

The managed layer uses `[DllImport("tealeaf_ffi")]` for P/Invoke. The .NET runtime resolves the native library through:

1. **NuGet runtimes folder** -- automatic for published apps
2. **Application directory** -- for self-contained deployments
3. **System library path** -- `PATH` (Windows), `LD_LIBRARY_PATH` (Linux), `DYLD_LIBRARY_PATH` (macOS)

## Deployment

### Framework-Dependent

```bash
dotnet publish -c Release
```

The native library is copied to the output directory automatically.

### Self-Contained

```bash
dotnet publish -c Release --self-contained -r win-x64
dotnet publish -c Release --self-contained -r linux-x64
dotnet publish -c Release --self-contained -r osx-arm64
```

### Docker

For Linux containers, use the appropriate runtime:

```dockerfile
FROM mcr.microsoft.com/dotnet/runtime:8.0
# Native library is included in the publish output
COPY --from=build /app/publish .
```

## Building Native Libraries from Source

If you need a platform not included in the NuGet package:

```bash
# Clone the repository
git clone https://github.com/krishjag/tealeaf.git
cd tealeaf

# Build the FFI library
cargo build --release --package tealeaf-ffi

# Output location
# Windows: target/release/tealeaf_ffi.dll
# Linux:   target/release/libtealeaf_ffi.so
# macOS:   target/release/libtealeaf_ffi.dylib
```

Place the built library in your application directory or system library path.

## Troubleshooting

### DllNotFoundException

The native library could not be found. Check:
1. The package includes your platform (`dotnet --info` to check RID)
2. For self-contained apps, ensure the correct `-r` flag is used
3. For manual builds, ensure the library is in the application directory

### BadImageFormatException

Architecture mismatch between the .NET runtime and native library. Ensure both are the same architecture (x64/ARM64).

### EntryPointNotFoundException

Version mismatch between the managed and native libraries. Ensure both are from the same release.
