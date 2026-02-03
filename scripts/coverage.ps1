<#
.SYNOPSIS
    Generates code coverage reports for the TeaLeaf project.

.DESCRIPTION
    Runs Rust coverage (cargo-llvm-cov) and .NET coverage (coverlet) and
    produces HTML reports for local viewing or lcov/cobertura for CI upload.

    Prerequisites:
    - Rust toolchain with llvm-tools-preview component
    - cargo-llvm-cov: cargo install cargo-llvm-cov
    - .NET SDK 10.0 (for .NET coverage)
    - Optional: dotnet tool install -g dotnet-reportgenerator-globaltool (for HTML)

.PARAMETER CI
    Generate lcov + cobertura for CI upload (no HTML).

.PARAMETER RustOnly
    Run Rust coverage only.

.PARAMETER DotnetOnly
    Run .NET coverage only.

.PARAMETER Open
    Open HTML reports in the default browser after generation.

.EXAMPLE
    ./scripts/coverage.ps1
    ./scripts/coverage.ps1 -CI
    ./scripts/coverage.ps1 -RustOnly -Open
    ./scripts/coverage.ps1 -DotnetOnly
#>

param(
    [switch]$CI,
    [switch]$RustOnly,
    [switch]$DotnetOnly,
    [switch]$Open
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$CoverageDir = Join-Path $RepoRoot "coverage"

# Create coverage output directory
if (-not (Test-Path $CoverageDir)) {
    New-Item -ItemType Directory -Path $CoverageDir -Force | Out-Null
}

# =========================================================================
# Rust Coverage
# =========================================================================
function Invoke-RustCoverage {
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host "  Rust Coverage (cargo-llvm-cov)" -ForegroundColor Cyan
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host ""

    # Verify cargo-llvm-cov is installed
    $llvmCov = Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue
    if (-not $llvmCov) {
        Write-Error "cargo-llvm-cov is not installed. Install with: cargo install cargo-llvm-cov"
        return
    }

    Push-Location $RepoRoot
    try {
        if ($CI) {
            Write-Host "Generating lcov output..."
            cargo llvm-cov --workspace `
                --exclude accuracy-benchmark `
                --lcov `
                --output-path "$CoverageDir\rust-lcov.info"
            Write-Host ""
            Write-Host "Rust lcov report: $CoverageDir\rust-lcov.info" -ForegroundColor Green
        } else {
            Write-Host "Generating HTML report..."
            cargo llvm-cov --workspace `
                --exclude accuracy-benchmark `
                --html `
                --output-dir "$CoverageDir\rust-html"

            # Also generate lcov for reference (reuse instrumented build)
            cargo llvm-cov --workspace `
                --exclude accuracy-benchmark `
                --lcov `
                --output-path "$CoverageDir\rust-lcov.info" `
                --no-run

            Write-Host ""
            Write-Host "Rust HTML report: $CoverageDir\rust-html\index.html" -ForegroundColor Green
            Write-Host "Rust lcov report: $CoverageDir\rust-lcov.info" -ForegroundColor Green
        }
    } finally {
        Pop-Location
    }
}

# =========================================================================
# .NET Coverage
# =========================================================================
function Invoke-DotnetCoverage {
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host "  .NET Coverage (coverlet)" -ForegroundColor Cyan
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host ""

    $DotnetDir = Join-Path $RepoRoot "bindings\dotnet"

    Write-Host "Building .NET solution..."
    dotnet build $DotnetDir -c Debug

    # Run TeaLeaf.Tests with coverage
    Write-Host ""
    Write-Host "Running TeaLeaf.Tests with coverage..."
    $tealeafTestsOutput = Join-Path $CoverageDir "dotnet-tealeaf-tests.cobertura.xml"
    dotnet test "$DotnetDir\TeaLeaf.Tests" `
        -c Debug `
        --no-build `
        /p:CollectCoverage=true `
        /p:CoverletOutputFormat=cobertura `
        /p:CoverletOutput="$tealeafTestsOutput" `
        "/p:Include=[TeaLeaf]*,[TeaLeaf.Annotations]*" `
        "/p:Exclude=[*.Tests]*,[*.Generators.Tests]*"

    # Run TeaLeaf.Generators.Tests with coverage
    Write-Host ""
    Write-Host "Running TeaLeaf.Generators.Tests with coverage..."
    $generatorsOutput = Join-Path $CoverageDir "dotnet-generators-tests.cobertura.xml"
    dotnet test "$DotnetDir\TeaLeaf.Generators.Tests" `
        -c Debug `
        --no-build `
        /p:CollectCoverage=true `
        /p:CoverletOutputFormat=cobertura `
        /p:CoverletOutput="$generatorsOutput" `
        "/p:Include=[TeaLeaf.Generators]*,[TeaLeaf.Annotations]*" `
        "/p:Exclude=[*.Tests]*"

    if (-not $CI) {
        # Local mode: try to generate HTML report
        $reportGen = Get-Command reportgenerator -ErrorAction SilentlyContinue
        if ($reportGen) {
            Write-Host ""
            Write-Host "Generating HTML report..."
            $reports = (Get-ChildItem "$CoverageDir\dotnet-*.cobertura.xml").FullName -join ";"
            reportgenerator `
                "-reports:$reports" `
                "-targetdir:$CoverageDir\dotnet-html" `
                "-reporttypes:Html"
            Write-Host ".NET HTML report: $CoverageDir\dotnet-html\index.html" -ForegroundColor Green
        } else {
            Write-Host ""
            Write-Host "Tip: Install reportgenerator for HTML reports:" -ForegroundColor Yellow
            Write-Host "  dotnet tool install -g dotnet-reportgenerator-globaltool" -ForegroundColor Yellow
        }
    }

    Write-Host ""
    Write-Host ".NET Cobertura reports in: $CoverageDir\" -ForegroundColor Green
}

# =========================================================================
# Main
# =========================================================================
Write-Host "TeaLeaf Coverage Report Generator" -ForegroundColor White
Write-Host "=================================" -ForegroundColor White
Write-Host ""

if (-not $DotnetOnly) {
    Invoke-RustCoverage
    Write-Host ""
}

if (-not $RustOnly) {
    Invoke-DotnetCoverage
    Write-Host ""
}

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Coverage generation complete!" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Output directory: $CoverageDir\" -ForegroundColor Green
Get-ChildItem $CoverageDir -ErrorAction SilentlyContinue | Format-Table Name, Length

if ($Open) {
    $rustReport = Join-Path $CoverageDir "rust-html\index.html"
    $dotnetReport = Join-Path $CoverageDir "dotnet-html\index.html"
    if (Test-Path $rustReport) { Start-Process $rustReport }
    if (Test-Path $dotnetReport) { Start-Process $dotnetReport }
}
