<#
.SYNOPSIS
    Synchronizes version and metadata from release.json to all project files.

.DESCRIPTION
    Reads release.json and updates:
    - Cargo.toml (workspace metadata)
    - bindings/dotnet/Pax/Pax.csproj (.NET package metadata)
    - README.md (footer version)
    - spec/PAX_SPEC.md (title version)

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
    return $content
}

# Update .NET csproj
$CsprojPath = Join-Path $RepoRoot "bindings/dotnet/Pax/Pax.csproj"
$DotnetPackage = $Release.packages.dotnet
Update-File -Path $CsprojPath -Description "Pax.csproj (.NET)" -Transform {
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
    $content = $content -replace '(\*PAX v)[^\s]+(\ —)', "`${1}$Version`${2}"
    return $content
}

# Update PAX_SPEC.md title version
$SpecPath = Join-Path $RepoRoot "spec/PAX_SPEC.md"
Update-File -Path $SpecPath -Description "spec/PAX_SPEC.md (title)" -Transform {
    param($content)
    $content = $content -replace '(# PAX Format Specification v)[^\s]+', "`${1}$Version"
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
