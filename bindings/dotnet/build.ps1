<#
.SYNOPSIS
    Build script for TeaLeaf .NET library with native dependencies.

.DESCRIPTION
    Compiles Rust native libraries for multiple platforms and packages
    the .NET library with all native dependencies.

.PARAMETER Configuration
    Build configuration: Debug or Release. Default is Release.

.PARAMETER SkipRust
    Skip Rust compilation (use existing native libraries).

.PARAMETER TargetRids
    Specific Runtime Identifiers to build. Default builds native platform only.
    Use -TargetRids all to build all supported platforms.
    Example: -TargetRids win-x64,win-arm64

.PARAMETER Test
    Run tests after building.

.PARAMETER Pack
    Create NuGet package after building.

.EXAMPLE
    .\build.ps1
    Build for native platform with default settings.

.EXAMPLE
    .\build.ps1 -Configuration Debug -TargetRids win-x64
    Build only win-x64 in Debug mode.

.EXAMPLE
    .\build.ps1 -TargetRids all -Pack
    Build all platforms and create NuGet package.

.EXAMPLE
    .\build.ps1 -Test -Pack
    Build, test, and package.
#>

param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release",

    [switch]$SkipRust,

    [string[]]$TargetRids,

    [switch]$Test,

    [switch]$Pack
)

$ErrorActionPreference = "Stop"

# Script directory and project paths
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "../..")
$DotnetDir = $ScriptDir
$TeaLeafDir = Join-Path $DotnetDir "TeaLeaf"
$RuntimesDir = Join-Path $TeaLeafDir "runtimes"

# Rust target mappings: RID -> (Rust target, library name)
$RustTargets = @{
    "win-x64"          = @{ Target = "x86_64-pc-windows-msvc"; Library = "tealeaf_ffi.dll" }
    "win-x86"          = @{ Target = "i686-pc-windows-msvc"; Library = "tealeaf_ffi.dll" }
    "win-arm64"        = @{ Target = "aarch64-pc-windows-msvc"; Library = "tealeaf_ffi.dll" }
    "linux-x64"        = @{ Target = "x86_64-unknown-linux-gnu"; Library = "libtealeaf_ffi.so" }
    "linux-arm64"      = @{ Target = "aarch64-unknown-linux-gnu"; Library = "libtealeaf_ffi.so" }
    "linux-musl-x64"   = @{ Target = "x86_64-unknown-linux-musl"; Library = "libtealeaf_ffi.so" }
    "linux-musl-arm64" = @{ Target = "aarch64-unknown-linux-musl"; Library = "libtealeaf_ffi.so" }
    "osx-x64"          = @{ Target = "x86_64-apple-darwin"; Library = "libtealeaf_ffi.dylib" }
    "osx-arm64"        = @{ Target = "aarch64-apple-darwin"; Library = "libtealeaf_ffi.dylib" }
}

# All supported RIDs
$AllRids = $RustTargets.Keys | Sort-Object

# Determine which RIDs to build
if ($TargetRids -contains "all") {
    $RidsToBuild = $AllRids
} elseif ($TargetRids) {
    $RidsToBuild = $TargetRids
} else {
    # Default: build only native platform on local builds
    $Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($Arch) {
        "X64"   { $RidsToBuild = @("win-x64") }
        "X86"   { $RidsToBuild = @("win-x86") }
        "Arm64" { $RidsToBuild = @("win-arm64") }
        default { $RidsToBuild = @("win-x64") }
    }
}

function Write-Step {
    param([string]$Message)
    Write-Host "`n=== $Message ===" -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host "  $Message" -ForegroundColor Gray
}

function Test-Command {
    param([string]$Command)
    $null = Get-Command $Command -ErrorAction SilentlyContinue
    return $?
}

function Install-RustTarget {
    param([string]$Target)

    $installed = (rustup target list --installed 2>$null) -contains $Target

    if (-not $installed) {
        Write-Host "  Installing Rust target: $Target" -ForegroundColor Yellow
        rustup target add $Target 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            return $false
        }
    }
    return $true
}

function Build-RustLibrary {
    param(
        [string]$Rid,
        [string]$Configuration
    )

    $targetInfo = $RustTargets[$Rid]
    if (-not $targetInfo) {
        Write-Warning "Unknown RID: $Rid"
        return $false
    }

    $rustTarget = $targetInfo.Target
    $libraryName = $targetInfo.Library

    Write-Host "  Building: $Rid ($rustTarget)" -ForegroundColor Cyan

    # Install target if needed
    if (-not (Install-RustTarget $rustTarget)) {
        Write-Warning "    Failed to install target $rustTarget. Skipping."
        return $false
    }

    # Build
    $cargoArgs = @("build", "--package", "tealeaf-ffi", "--target", $rustTarget)
    if ($Configuration -eq "Release") {
        $cargoArgs += "--release"
    }

    Push-Location $RepoRoot
    try {
        # Run cargo and let it output directly (cargo uses stderr for progress)
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "    Cargo build failed for $rustTarget"
            return $false
        }
    } finally {
        Pop-Location
    }

    # Copy output
    $buildProfile = if ($Configuration -eq "Release") { "release" } else { "debug" }
    $sourcePath = Join-Path $RepoRoot "target/$rustTarget/$buildProfile/$libraryName"
    $destDir = Join-Path $RuntimesDir "$Rid/native"
    $destPath = Join-Path $destDir $libraryName

    if (Test-Path $sourcePath) {
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        Copy-Item $sourcePath $destPath -Force
        Write-Info "Copied: $destPath"
        return $true
    } else {
        Write-Warning "    Library not found at: $sourcePath"
        return $false
    }
}

# Main build process
Write-Step "TeaLeaf .NET Build Script"
Write-Host "Configuration: $Configuration"
Write-Host "Target RIDs: $($RidsToBuild -join ', ')"

# Check prerequisites
Write-Step "Checking Prerequisites"

if (-not (Test-Command "cargo")) {
    throw "Rust/Cargo not found. Please install from https://rustup.rs"
}
Write-Host "  Cargo: $(cargo --version)"

if (-not (Test-Command "dotnet")) {
    throw ".NET SDK not found. Please install from https://dot.net"
}
Write-Host "  .NET: $(dotnet --version)"

# Build Rust libraries
if (-not $SkipRust) {
    Write-Step "Building Rust Native Libraries"

    $successCount = 0
    $failedRids = @()

    foreach ($rid in $RidsToBuild) {
        if (Build-RustLibrary -Rid $rid -Configuration $Configuration) {
            $successCount++
        } else {
            $failedRids += $rid
        }
    }

    Write-Host ""
    if ($successCount -gt 0) {
        Write-Host "Built $successCount of $($RidsToBuild.Count) native libraries" -ForegroundColor Green
    }
    if ($failedRids.Count -gt 0) {
        Write-Warning "Failed to build: $($failedRids -join ', ')"
    }
    if ($successCount -eq 0) {
        throw "No native libraries were built successfully"
    }
} else {
    Write-Host "Skipping Rust build (using existing libraries)" -ForegroundColor Yellow
}

# Build .NET project
Write-Step "Building .NET Project"

dotnet build $TeaLeafDir -c $Configuration
if ($LASTEXITCODE -ne 0) {
    throw ".NET build failed"
}

# Run tests if requested
if ($Test) {
    Write-Step "Running Tests"

    # Ensure native library is in test output directory
    $testProject = Join-Path $DotnetDir "TeaLeaf.Tests"
    if (Test-Path $testProject) {
        # Find the first available native library
        $nativeLib = Get-ChildItem -Path $RuntimesDir -Recurse -Filter "tealeaf_ffi.dll" | Select-Object -First 1
        if ($nativeLib) {
            $testBinDir = Join-Path $testProject "bin/$Configuration/net8.0"
            if (Test-Path $testBinDir) {
                Copy-Item $nativeLib.FullName $testBinDir -Force
            }
            $testBinDir10 = Join-Path $testProject "bin/$Configuration/net10.0"
            if (Test-Path $testBinDir10) {
                Copy-Item $nativeLib.FullName $testBinDir10 -Force
            }
        }

        dotnet test $testProject -c $Configuration --no-build
        if ($LASTEXITCODE -ne 0) {
            throw "Tests failed"
        }
    } else {
        Write-Warning "Test project not found at: $testProject"
    }
}

# Create NuGet package
if ($Pack) {
    Write-Step "Creating NuGet Package"

    $artifactsDir = Join-Path $DotnetDir "artifacts"
    New-Item -ItemType Directory -Path $artifactsDir -Force | Out-Null

    dotnet pack $TeaLeafDir -c $Configuration --no-build -o $artifactsDir
    if ($LASTEXITCODE -ne 0) {
        throw "NuGet pack failed"
    }

    $packages = Get-ChildItem -Path $artifactsDir -Filter "*.nupkg" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($packages) {
        Write-Host "`nPackage created: $($packages.Name)" -ForegroundColor Green
        Write-Info "Location: $($packages.FullName)"
    }
}

Write-Step "Build Complete"

# Show summary of native libraries
Write-Host "`nNative libraries included:"
$libs = Get-ChildItem -Path $RuntimesDir -Recurse -File -ErrorAction SilentlyContinue
if ($libs) {
    foreach ($lib in $libs) {
        $relativePath = $lib.FullName.Replace($TeaLeafDir + "\", "").Replace($TeaLeafDir + "/", "")
        Write-Info $relativePath
    }
} else {
    Write-Warning "No native libraries found in runtimes folder"
}
