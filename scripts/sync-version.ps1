<#
.SYNOPSIS
    Synchronizes version and metadata from release.json to all project files.

.DESCRIPTION
    Reads release.json and updates:
    - Cargo.toml (workspace metadata + dependency versions)
    - bindings/dotnet/TeaLeaf/TeaLeaf.csproj (.NET package metadata)
    - bindings/dotnet/TeaLeaf.Annotations/TeaLeaf.Annotations.csproj (version)
    - bindings/dotnet/TeaLeaf.Generators/TeaLeaf.Generators.csproj (version)
    - bindings/dotnet/BUILD.md (nupkg filename)
    - bindings/dotnet/docfx.json (footer version)
    - tealeaf-ffi/cbindgen.toml (header version)
    - CLAUDE.md (current version)
    - README.md (footer version)
    - spec/TEALEAF_SPEC.md (title version)
    - docs-site/src/introduction.md (version badge)
    - docs-site/src/appendix/comparison-matrix.md (version mention)
    - docs-site/src/getting-started/installation.md (CLI output + cargo version)
    - docs-site/src/rust/overview.md (cargo version)
    - docs-site/src/rust/derive-macros.md (cargo version)
    - docs-site/src/ffi/api-reference.md (version example)

.PARAMETER DryRun
    Show what would be changed without making changes.

.EXAMPLE
    ./scripts/sync-version.ps1
    ./scripts/sync-version.ps1 -DryRun
#>

param(
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

# Read release metadata
$ReleasePath = Join-Path $RepoRoot "release.json"
if (-not (Test-Path $ReleasePath)) {
    Write-Error "release.json not found at $ReleasePath"
    exit 1
}

$Release = Get-Content $ReleasePath -Raw | ConvertFrom-Json
$Version = $Release.version
$Authors = $Release.authors
$License = $Release.license
$Repository = $Release.repository

Write-Host "Release Metadata:" -ForegroundColor Cyan
Write-Host "  Version:    $Version"
Write-Host "  Authors:    $Authors"
Write-Host "  License:    $License"
Write-Host "  Repository: $Repository"
Write-Host ""

# Track if any updates were needed
$script:UpdatesNeeded = $false

# Helper function to update file
function Update-File {
    param(
        [string]$Path,
        [string]$Description,
        [scriptblock]$Transform
    )

    if (-not (Test-Path $Path)) {
        Write-Warning "File not found: $Path"
        return
    }

    $content = Get-Content $Path -Raw
    $newContent = & $Transform $content

    if ($content -ne $newContent) {
        $script:UpdatesNeeded = $true
        if ($DryRun) {
            Write-Host "[DRY RUN] Would update: $Description" -ForegroundColor Yellow
        } else {
            $newContent | Set-Content $Path -NoNewline
            Write-Host "Updated: $Description" -ForegroundColor Green
        }
    } else {
        Write-Host "No changes: $Description" -ForegroundColor DarkGray
    }
}

# Update Cargo.toml
$CargoPath = Join-Path $RepoRoot "Cargo.toml"
Update-File -Path $CargoPath -Description "Cargo.toml (workspace)" -Transform {
    param($content)
    $content = $content -replace '(?m)^(version\s*=\s*")[^"]*(")', "`${1}$Version`${2}"
    $content = $content -replace '(?m)^(authors\s*=\s*\[")[^"]*("\])', "`${1}$Authors`${2}"
    $content = $content -replace '(?m)^(license\s*=\s*")[^"]*(")', "`${1}$License`${2}"
    $content = $content -replace '(?m)^(repository\s*=\s*")[^"]*(")', "`${1}$Repository`${2}"
    # Update workspace dependency version specifiers (required for crates.io publish)
    $content = $content -replace '(tealeaf-core = \{ path = "tealeaf-core", version = ")[^"]*(")', "`${1}$Version`${2}"
    $content = $content -replace '(tealeaf-derive = \{ path = "tealeaf-derive", version = ")[^"]*(")', "`${1}$Version`${2}"
    return $content
}

# Update .NET csproj
$CsprojPath = Join-Path $RepoRoot "bindings/dotnet/TeaLeaf/TeaLeaf.csproj"
$DotnetPackage = $Release.packages.dotnet
Update-File -Path $CsprojPath -Description "TeaLeaf.csproj (.NET)" -Transform {
    param($content)
    $content = $content -replace '(<Version>)[^<]*(</Version>)', "`${1}$Version`${2}"
    $content = $content -replace '(<Authors>)[^<]*(</Authors>)', "`${1}$Authors`${2}"
    $content = $content -replace '(<PackageLicenseExpression>)[^<]*(</PackageLicenseExpression>)', "`${1}$License`${2}"
    $content = $content -replace '(<RepositoryUrl>)[^<]*(</RepositoryUrl>)', "`${1}$Repository`${2}"
    $content = $content -replace '(<PackageProjectUrl>)[^<]*(</PackageProjectUrl>)', "`${1}$Repository`${2}"
    if ($DotnetPackage.description) {
        $desc = $DotnetPackage.description
        $content = $content -replace '(<Description>)[^<]*(</Description>)', "`${1}$desc`${2}"
    }
    return $content
}

# Update README.md footer version
$ReadmePath = Join-Path $RepoRoot "README.md"
Update-File -Path $ReadmePath -Description "README.md (footer)" -Transform {
    param($content)
    $content = $content -replace '(\*TeaLeaf v)[^\s]+(\ —)', "`${1}$Version`${2}"
    return $content
}

# Update TEALEAF_SPEC.md title version
$SpecPath = Join-Path $RepoRoot "spec/TEALEAF_SPEC.md"
Update-File -Path $SpecPath -Description "spec/TEALEAF_SPEC.md (title)" -Transform {
    param($content)
    $content = $content -replace '(# TeaLeaf Format Specification v)[^\s]+', "`${1}$Version"
    return $content
}

# Update Rust source version constants
$TypesPath = Join-Path $RepoRoot "tealeaf-core/src/types.rs"
Update-File -Path $TypesPath -Description "tealeaf-core/src/types.rs (VERSION)" -Transform {
    param($content)
    $content = $content -replace '(pub const VERSION: &str = ")[^"]*(")', "`${1}$Version`${2}"
    return $content
}

$FfiLibPath = Join-Path $RepoRoot "tealeaf-ffi/src/lib.rs"
Update-File -Path $FfiLibPath -Description "tealeaf-ffi/src/lib.rs (tl_version)" -Transform {
    param($content)
    $content = $content -replace '(static VERSION: &\[u8\] = b")[^\\]*(\\0")', "`${1}$Version`${2}"
    $content = $content -replace '(assert_eq!\(version, ")[^"]*(")', "`${1}$Version`${2}"
    return $content
}

# Update generated C header
$HeaderPath = Join-Path $RepoRoot "tealeaf-ffi/tealeaf.h"
Update-File -Path $HeaderPath -Description "tealeaf-ffi/tealeaf.h (header version)" -Transform {
    param($content)
    $content = $content -replace '(\* Version: )[^\s]+ (\(Request)', "`${1}$Version `${2}"
    return $content
}

# Update CLAUDE.md version
$ClaudeMdPath = Join-Path $RepoRoot "CLAUDE.md"
Update-File -Path $ClaudeMdPath -Description "CLAUDE.md (current version)" -Transform {
    param($content)
    $content = $content -replace '(Current version: \*\*)[^\*]+(\*\*)', "`${1}$Version`${2}"
    return $content
}

# Update .NET Annotations csproj
$AnnotationsPath = Join-Path $RepoRoot "bindings/dotnet/TeaLeaf.Annotations/TeaLeaf.Annotations.csproj"
Update-File -Path $AnnotationsPath -Description "TeaLeaf.Annotations.csproj (version)" -Transform {
    param($content)
    $content = $content -replace '(<Version>)[^<]*(</Version>)', "`${1}$Version`${2}"
    return $content
}

# Update .NET Generators csproj
$GeneratorsPath = Join-Path $RepoRoot "bindings/dotnet/TeaLeaf.Generators/TeaLeaf.Generators.csproj"
Update-File -Path $GeneratorsPath -Description "TeaLeaf.Generators.csproj (version)" -Transform {
    param($content)
    $content = $content -replace '(<Version>)[^<]*(</Version>)', "`${1}$Version`${2}"
    return $content
}

# Update cbindgen.toml header version
$CbindgenPath = Join-Path $RepoRoot "tealeaf-ffi/cbindgen.toml"
Update-File -Path $CbindgenPath -Description "cbindgen.toml (header version)" -Transform {
    param($content)
    $content = $content -replace '(\* Version: )[^\s]+ (\(Request)', "`${1}$Version `${2}"
    return $content
}

# Update docfx.json footer
$DocfxPath = Join-Path $RepoRoot "bindings/dotnet/docfx.json"
Update-File -Path $DocfxPath -Description "docfx.json (footer version)" -Transform {
    param($content)
    $content = $content -replace '(TeaLeaf v)[^\s]+ (—)', "`${1}$Version `${2}"
    return $content
}

# Update BUILD.md nupkg filename
$BuildMdPath = Join-Path $RepoRoot "bindings/dotnet/BUILD.md"
Update-File -Path $BuildMdPath -Description "BUILD.md (nupkg filename)" -Transform {
    param($content)
    $content = $content -replace '(TeaLeaf\.)[^\s]+(\.nupkg)', "`${1}$Version`${2}"
    return $content
}

# Update docs-site version references
$IntroPath = Join-Path $RepoRoot "docs-site/src/introduction.md"
Update-File -Path $IntroPath -Description "introduction.md (version badge)" -Transform {
    param($content)
    $content = $content -replace '(version-badge">v)[^<]*(</span>)', "`${1}$Version`${2}"
    return $content
}

$ComparisonPath = Join-Path $RepoRoot "docs-site/src/appendix/comparison-matrix.md"
Update-File -Path $ComparisonPath -Description "comparison-matrix.md (version)" -Transform {
    param($content)
    $content = $content -replace '(young format \(v)[^\)]*(\))', "`${1}$Version`${2}"
    return $content
}

$InstallPath = Join-Path $RepoRoot "docs-site/src/getting-started/installation.md"
Update-File -Path $InstallPath -Description "installation.md (version refs)" -Transform {
    param($content)
    $content = $content -replace '(# tealeaf )[^\s]+', "`${1}$Version"
    $content = $content -replace '(tealeaf-core = \{ version = ")[^"]*(")', "`${1}$Version`${2}"
    return $content
}

$RustOverviewPath = Join-Path $RepoRoot "docs-site/src/rust/overview.md"
Update-File -Path $RustOverviewPath -Description "rust/overview.md (cargo version)" -Transform {
    param($content)
    $content = $content -replace '(tealeaf-core = \{ version = ")[^"]*(")', "`${1}$Version`${2}"
    return $content
}

$DerivePath = Join-Path $RepoRoot "docs-site/src/rust/derive-macros.md"
Update-File -Path $DerivePath -Description "rust/derive-macros.md (cargo version)" -Transform {
    param($content)
    $content = $content -replace '(tealeaf-core = \{ version = ")[^"]*(")', "`${1}$Version`${2}"
    return $content
}

$FfiRefPath = Join-Path $RepoRoot "docs-site/src/ffi/api-reference.md"
Update-File -Path $FfiRefPath -Description "ffi/api-reference.md (version example)" -Transform {
    param($content)
    $content = $content -replace '(e\.g\., `")[^"]*(")', "`${1}$Version`${2}"
    return $content
}

Write-Host ""
if ($DryRun) {
    Write-Host "Dry run complete. No files were modified." -ForegroundColor Yellow
    # Exit with code 2 if updates are needed (for validation)
    if ($script:UpdatesNeeded) {
        exit 2
    }
    exit 0
} else {
    Write-Host "Version sync complete!" -ForegroundColor Green
    exit 0
}
