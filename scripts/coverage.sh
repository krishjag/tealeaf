#!/bin/bash
#
# Generates code coverage reports for the TeaLeaf project.
#
# Prerequisites:
#   - Rust toolchain with llvm-tools-preview component
#   - cargo-llvm-cov: cargo install cargo-llvm-cov
#   - .NET SDK 10.0 (for .NET coverage)
#   - Optional: dotnet tool install -g dotnet-reportgenerator-globaltool (for HTML)
#
# Usage:
#   ./scripts/coverage.sh              # Generate HTML reports (default)
#   ./scripts/coverage.sh --ci         # Generate lcov + cobertura for CI upload
#   ./scripts/coverage.sh --rust-only  # Rust coverage only
#   ./scripts/coverage.sh --dotnet-only # .NET coverage only
#   ./scripts/coverage.sh --open       # Generate HTML and open in browser

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
COVERAGE_DIR="$REPO_ROOT/coverage"

# Parse arguments
CI_MODE=false
RUST_ONLY=false
DOTNET_ONLY=false
OPEN_REPORT=false

for arg in "$@"; do
    case $arg in
        --ci) CI_MODE=true ;;
        --rust-only) RUST_ONLY=true ;;
        --dotnet-only) DOTNET_ONLY=true ;;
        --open) OPEN_REPORT=true ;;
        --help|-h)
            echo "Usage: $0 [--ci] [--rust-only] [--dotnet-only] [--open]"
            echo ""
            echo "Options:"
            echo "  --ci           Generate lcov/cobertura output for CI (no HTML)"
            echo "  --rust-only    Run Rust coverage only"
            echo "  --dotnet-only  Run .NET coverage only"
            echo "  --open         Open HTML report in browser after generation"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg"
            exit 1
            ;;
    esac
done

# Create coverage output directory
mkdir -p "$COVERAGE_DIR"

# =========================================================================
# Rust Coverage
# =========================================================================
run_rust_coverage() {
    echo "============================================"
    echo "  Rust Coverage (cargo-llvm-cov)"
    echo "============================================"
    echo ""

    # Verify cargo-llvm-cov is installed
    if ! command -v cargo-llvm-cov &> /dev/null; then
        echo "Error: cargo-llvm-cov is not installed." >&2
        echo "Install with: cargo install cargo-llvm-cov" >&2
        echo "  or: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov" >&2
        exit 1
    fi

    cd "$REPO_ROOT"

    if [[ "$CI_MODE" == "true" ]]; then
        # CI mode: generate lcov for Codecov upload
        echo "Generating lcov output..."
        cargo llvm-cov --workspace \
            --exclude accuracy-benchmark \
            --lcov \
            --output-path "$COVERAGE_DIR/rust-lcov.info"
        echo ""
        echo "Rust lcov report: $COVERAGE_DIR/rust-lcov.info"
    else
        # Local mode: generate HTML report
        echo "Generating HTML report..."
        cargo llvm-cov --workspace \
            --exclude accuracy-benchmark \
            --html \
            --output-dir "$COVERAGE_DIR/rust-html"

        # Also generate lcov for reference (reuse instrumented build)
        cargo llvm-cov --workspace \
            --exclude accuracy-benchmark \
            --lcov \
            --output-path "$COVERAGE_DIR/rust-lcov.info" \
            --no-run

        echo ""
        echo "Rust HTML report: $COVERAGE_DIR/rust-html/index.html"
        echo "Rust lcov report: $COVERAGE_DIR/rust-lcov.info"
    fi
}

# =========================================================================
# .NET Coverage
# =========================================================================
run_dotnet_coverage() {
    echo "============================================"
    echo "  .NET Coverage (coverlet)"
    echo "============================================"
    echo ""

    DOTNET_DIR="$REPO_ROOT/bindings/dotnet"

    # Build in Debug mode for coverage
    echo "Building .NET solution..."
    dotnet build "$DOTNET_DIR" -c Debug

    # Run TeaLeaf.Tests with coverage
    echo ""
    echo "Running TeaLeaf.Tests with coverage..."
    dotnet test "$DOTNET_DIR/TeaLeaf.Tests" \
        -c Debug \
        --no-build \
        /p:CollectCoverage=true \
        /p:CoverletOutputFormat=cobertura \
        /p:CoverletOutput="$COVERAGE_DIR/dotnet-tealeaf-tests.cobertura.xml" \
        '/p:Include=[TeaLeaf]*,[TeaLeaf.Annotations]*' \
        '/p:Exclude=[*.Tests]*,[*.Generators.Tests]*'

    # Run TeaLeaf.Generators.Tests with coverage
    echo ""
    echo "Running TeaLeaf.Generators.Tests with coverage..."
    dotnet test "$DOTNET_DIR/TeaLeaf.Generators.Tests" \
        -c Debug \
        --no-build \
        /p:CollectCoverage=true \
        /p:CoverletOutputFormat=cobertura \
        /p:CoverletOutput="$COVERAGE_DIR/dotnet-generators-tests.cobertura.xml" \
        '/p:Include=[TeaLeaf.Generators]*,[TeaLeaf.Annotations]*' \
        '/p:Exclude=[*.Tests]*'

    if [[ "$CI_MODE" != "true" ]]; then
        # Local mode: generate HTML report using reportgenerator
        if command -v reportgenerator &> /dev/null; then
            echo ""
            echo "Generating HTML report..."
            reportgenerator \
                -reports:"$COVERAGE_DIR/dotnet-*.cobertura.xml" \
                -targetdir:"$COVERAGE_DIR/dotnet-html" \
                -reporttypes:Html
            echo ".NET HTML report: $COVERAGE_DIR/dotnet-html/index.html"
        else
            echo ""
            echo "Tip: Install reportgenerator for HTML reports:"
            echo "  dotnet tool install -g dotnet-reportgenerator-globaltool"
        fi
    fi

    echo ""
    echo ".NET Cobertura reports in: $COVERAGE_DIR/"
}

# =========================================================================
# Main
# =========================================================================
echo "TeaLeaf Coverage Report Generator"
echo "================================="
echo ""

if [[ "$DOTNET_ONLY" != "true" ]]; then
    run_rust_coverage
    echo ""
fi

if [[ "$RUST_ONLY" != "true" ]]; then
    run_dotnet_coverage
    echo ""
fi

echo "============================================"
echo "  Coverage generation complete!"
echo "============================================"
echo "Output directory: $COVERAGE_DIR/"
ls -la "$COVERAGE_DIR/" 2>/dev/null || true

if [[ "$OPEN_REPORT" == "true" ]]; then
    if [[ -f "$COVERAGE_DIR/rust-html/index.html" ]]; then
        xdg-open "$COVERAGE_DIR/rust-html/index.html" 2>/dev/null || \
        open "$COVERAGE_DIR/rust-html/index.html" 2>/dev/null || true
    fi
    if [[ -f "$COVERAGE_DIR/dotnet-html/index.html" ]]; then
        xdg-open "$COVERAGE_DIR/dotnet-html/index.html" 2>/dev/null || \
        open "$COVERAGE_DIR/dotnet-html/index.html" 2>/dev/null || true
    fi
fi
