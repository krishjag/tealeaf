#!/bin/bash
#
# Synchronizes version and metadata from release.json to all project files.
#
# Usage:
#   ./scripts/sync-version.sh           # Apply changes
#   ./scripts/sync-version.sh --dry-run # Show what would change

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

DRY_RUN=false
if [[ "$1" == "--dry-run" ]]; then
    DRY_RUN=true
fi

# Read release metadata
RELEASE_FILE="$REPO_ROOT/release.json"
if [[ ! -f "$RELEASE_FILE" ]]; then
    echo "Error: release.json not found at $RELEASE_FILE" >&2
    exit 1
fi

# Parse JSON (requires jq)
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required but not installed." >&2
    echo "Install with: apt-get install jq (Linux) or brew install jq (macOS)" >&2
    exit 1
fi

VERSION=$(jq -r '.version' "$RELEASE_FILE")
AUTHORS=$(jq -r '.authors' "$RELEASE_FILE")
LICENSE=$(jq -r '.license' "$RELEASE_FILE")
REPOSITORY=$(jq -r '.repository' "$RELEASE_FILE")
DOTNET_DESCRIPTION=$(jq -r '.packages.dotnet.description' "$RELEASE_FILE")

# Benchmark metrics
BENCH_DATE=$(jq -r '.benchmark.date' "$RELEASE_FILE")
BENCH_TASKS=$(jq -r '.benchmark.tasks' "$RELEASE_FILE")
BENCH_DOMAINS=$(jq -r '.benchmark.domains' "$RELEASE_FILE")
TL_SAVINGS=$(jq -r '.benchmark.tl_input_savings_pct' "$RELEASE_FILE")
TOON_SAVINGS=$(jq -r '.benchmark.toon_input_savings_pct' "$RELEASE_FILE")
ACC_ANTHROPIC_TL=$(jq -r '.benchmark.accuracy.anthropic_tl' "$RELEASE_FILE")
ACC_ANTHROPIC_JSON=$(jq -r '.benchmark.accuracy.anthropic_json' "$RELEASE_FILE")
ACC_ANTHROPIC_TOON=$(jq -r '.benchmark.accuracy.anthropic_toon' "$RELEASE_FILE")
ACC_OPENAI_TL=$(jq -r '.benchmark.accuracy.openai_tl' "$RELEASE_FILE")
ACC_OPENAI_JSON=$(jq -r '.benchmark.accuracy.openai_json' "$RELEASE_FILE")
ACC_OPENAI_TOON=$(jq -r '.benchmark.accuracy.openai_toon' "$RELEASE_FILE")

echo "Release Metadata:"
echo "  Version:    $VERSION"
echo "  Authors:    $AUTHORS"
echo "  License:    $LICENSE"
echo "  Repository: $REPOSITORY"
echo "  Benchmark:  $BENCH_DATE ($BENCH_TASKS tasks, $BENCH_DOMAINS domains, TL saves ~${TL_SAVINGS}%)"
echo ""

# Track if any updates are needed
UPDATES_NEEDED=false

# Helper function to check and update file
update_file() {
    local file="$1"
    local description="$2"
    local pattern="$3"
    local replacement="$4"

    if [[ ! -f "$file" ]]; then
        echo "Warning: File not found: $file"
        return
    fi

    # Check if pattern matches and replacement differs
    local current=$(grep -E "$pattern" "$file" 2>/dev/null | head -1 || true)
    local would_be=$(echo "$current" | sed -E "s|$pattern|$replacement|" || true)

    if [[ "$current" != "$would_be" && -n "$current" ]]; then
        UPDATES_NEEDED=true
        if [[ "$DRY_RUN" == "true" ]]; then
            echo "[DRY RUN] Would update: $description"
        else
            sed -i.bak -E "s|$pattern|$replacement|" "$file"
            rm -f "$file.bak"
            echo "Updated: $description"
        fi
    else
        echo "No changes: $description"
    fi
}

# Update Cargo.toml
CARGO_FILE="$REPO_ROOT/Cargo.toml"
update_file "$CARGO_FILE" "Cargo.toml (workspace version)" \
    "^(version = \")[^\"]*(\")$" "\1$VERSION\2"

# Update workspace dependency version specifiers (required for crates.io publish)
update_file "$CARGO_FILE" "Cargo.toml (tealeaf-core dep version)" \
    "(tealeaf-core = \{ path = \"tealeaf-core\", version = \")[^\"]*(\")" "\1$VERSION\2"
update_file "$CARGO_FILE" "Cargo.toml (tealeaf-derive dep version)" \
    "(tealeaf-derive = \{ path = \"tealeaf-derive\", version = \")[^\"]*(\")" "\1$VERSION\2"

# Update .NET csproj
CSPROJ_FILE="$REPO_ROOT/bindings/dotnet/TeaLeaf/TeaLeaf.csproj"
update_file "$CSPROJ_FILE" "TeaLeaf.csproj (version)" \
    "(<Version>)[^<]*(</Version>)" "\1$VERSION\2"

# Update README.md footer version
README_FILE="$REPO_ROOT/README.md"
update_file "$README_FILE" "README.md (footer)" \
    "(\*TeaLeaf v)[^ ]+( —)" "\1$VERSION\2"

# Update TEALEAF_SPEC.md title version
SPEC_FILE="$REPO_ROOT/spec/TEALEAF_SPEC.md"
update_file "$SPEC_FILE" "spec/TEALEAF_SPEC.md (title)" \
    "(# TeaLeaf Format Specification v)[^ ]+" "\1$VERSION"
update_file "$SPEC_FILE" "spec/TEALEAF_SPEC.md (footer)" \
    "(\*TeaLeaf Format Specification v)[^*]+(\*)" "\1$VERSION\2"

# Update Rust source version constants
update_file "$REPO_ROOT/tealeaf-core/src/types.rs" "tealeaf-core/src/types.rs (VERSION)" \
    "(pub const VERSION: \&str = \")[^\"]*(\";)" "\1$VERSION\2"

update_file "$REPO_ROOT/tealeaf-ffi/src/lib.rs" "tealeaf-ffi/src/lib.rs (tl_version)" \
    "(static VERSION: \&\[u8\] = b\")[^\\\\]*(\\\\0\")" "\1$VERSION\2"

update_file "$REPO_ROOT/tealeaf-ffi/src/lib.rs" "tealeaf-ffi/src/lib.rs (version test)" \
    "(assert_eq!\(version, \")[^\"]*(\")" "\1$VERSION\2"

# Update generated C header
update_file "$REPO_ROOT/tealeaf-ffi/tealeaf.h" "tealeaf-ffi/tealeaf.h (header version)" \
    "(\* Version: )[^ ]+ (\(Request)" "\1$VERSION \2"

# Update CLAUDE.md version
update_file "$REPO_ROOT/CLAUDE.md" "CLAUDE.md (current version)" \
    "(Current version: \*\*)[^\*]+(\*\*)" "\1$VERSION\2"

# Update .NET Annotations csproj
update_file "$REPO_ROOT/bindings/dotnet/TeaLeaf.Annotations/TeaLeaf.Annotations.csproj" \
    "TeaLeaf.Annotations.csproj (version)" \
    "(<Version>)[^<]*(</Version>)" "\1$VERSION\2"

# Update .NET Generators csproj
update_file "$REPO_ROOT/bindings/dotnet/TeaLeaf.Generators/TeaLeaf.Generators.csproj" \
    "TeaLeaf.Generators.csproj (version)" \
    "(<Version>)[^<]*(</Version>)" "\1$VERSION\2"

# Update cbindgen.toml header version
update_file "$REPO_ROOT/tealeaf-ffi/cbindgen.toml" "cbindgen.toml (header version)" \
    "(\* Version: )[^ ]+ (\(Request)" "\1$VERSION \2"

# Update docfx.json footer
update_file "$REPO_ROOT/bindings/dotnet/docfx.json" "docfx.json (footer version)" \
    "(TeaLeaf v)[^ ]+ (—)" "\1$VERSION \2"

# Update BUILD.md nupkg filename
update_file "$REPO_ROOT/bindings/dotnet/BUILD.md" "BUILD.md (nupkg filename)" \
    "(TeaLeaf\.)[^ ]+(\.nupkg)" "\1$VERSION\2"

# Update tealeaf-core/README.md cargo versions
update_file "$REPO_ROOT/tealeaf-core/README.md" \
    "tealeaf-core/README.md (cargo plain version)" \
    "(tealeaf-core = \")[^\"]*(\")" "\1$VERSION\2"
update_file "$REPO_ROOT/tealeaf-core/README.md" \
    "tealeaf-core/README.md (cargo features version)" \
    "(tealeaf-core = \{ version = \")[^\"]*(\")" "\1$VERSION\2"

# Update docs-site version references
update_file "$REPO_ROOT/docs-site/src/introduction.md" "introduction.md (version badge)" \
    "(version-badge\">v)[^<]*(</span>)" "\1$VERSION\2"

update_file "$REPO_ROOT/docs-site/src/appendix/comparison-matrix.md" \
    "comparison-matrix.md (version)" \
    "(young format \(v)[^\)]*(\))" "\1$VERSION\2"

update_file "$REPO_ROOT/docs-site/src/getting-started/installation.md" \
    "installation.md (CLI version)" \
    "(# tealeaf )[^ ]+" "\1$VERSION"

update_file "$REPO_ROOT/docs-site/src/getting-started/installation.md" \
    "installation.md (cargo version)" \
    "(tealeaf-core = \{ version = \")[^\"]*(\")" "\1$VERSION\2"

update_file "$REPO_ROOT/docs-site/src/rust/overview.md" \
    "rust/overview.md (cargo version)" \
    "(tealeaf-core = \{ version = \")[^\"]*(\")" "\1$VERSION\2"

update_file "$REPO_ROOT/docs-site/src/rust/derive-macros.md" \
    "rust/derive-macros.md (cargo version)" \
    "(tealeaf-core = \{ version = \")[^\"]*(\")" "\1$VERSION\2"

update_file "$REPO_ROOT/docs-site/src/ffi/api-reference.md" \
    "ffi/api-reference.md (version example)" \
    "(e\.g\., \`\")[^\"]*(\")" "\1$VERSION\2"

# ── Benchmark metric propagation ─────────────────────────────────────────────

# README.md -- headline savings
update_file "$REPO_ROOT/README.md" "README.md (benchmark headline)" \
    "~[0-9]+% fewer input tokens than JSON" "~${TL_SAVINGS}% fewer input tokens than JSON"

# README.md -- savings bullet
update_file "$REPO_ROOT/README.md" "README.md (benchmark savings bullet)" \
    "(\*\*~)[0-9]+(% fewer input tokens\*\* on real-world data)" "\1${TL_SAVINGS}\2"

# README.md -- accuracy table (Anthropic row)
update_file "$REPO_ROOT/README.md" "README.md (Anthropic accuracy)" \
    "(\| Anthropic accuracy \| )[0-9.]+( \| )[0-9.]+( \| )[0-9.]+( \|)" "\1${ACC_ANTHROPIC_TL}\2${ACC_ANTHROPIC_JSON}\3${ACC_ANTHROPIC_TOON}\4"

# README.md -- accuracy table (OpenAI row)
update_file "$REPO_ROOT/README.md" "README.md (OpenAI accuracy)" \
    "(\| OpenAI accuracy \| )[0-9.]+( \| )[0-9.]+( \| )[0-9.]+( \|)" "\1${ACC_OPENAI_TL}\2${ACC_OPENAI_JSON}\3${ACC_OPENAI_TOON}\4"

# README.md -- input savings row
update_file "$REPO_ROOT/README.md" "README.md (input savings row)" \
    "(\| Input token savings \| \*\*-)[0-9]+(%\*\* \| baseline \| \*\*-)[0-9]+(%\*\* \|)" "\1${TL_SAVINGS}\2${TOON_SAVINGS}\3"

# CLAUDE.md -- savings bullet
update_file "$REPO_ROOT/CLAUDE.md" "CLAUDE.md (benchmark savings)" \
    "(\*\*~)[0-9]+(% fewer input tokens\*\* on real-world data)" "\1${TL_SAVINGS}\2"

# CLAUDE.md -- accuracy example
update_file "$REPO_ROOT/CLAUDE.md" "CLAUDE.md (accuracy example)" \
    "(e\.g\., TL )[0-9.]+( vs JSON )[0-9.]+( on Anthropic)" "\1${ACC_ANTHROPIC_TL}\2${ACC_ANTHROPIC_JSON}\3"

# tealeaf-core/README.md -- headline
update_file "$REPO_ROOT/tealeaf-core/README.md" "tealeaf-core/README.md (benchmark headline)" \
    "~[0-9]+% fewer input tokens than JSON" "~${TL_SAVINGS}% fewer input tokens than JSON"

# docs-site/src/introduction.md -- headline + LLM section
update_file "$REPO_ROOT/docs-site/src/introduction.md" "introduction.md (benchmark headline)" \
    "~[0-9]+% fewer input tokens than JSON" "~${TL_SAVINGS}% fewer input tokens than JSON"

update_file "$REPO_ROOT/docs-site/src/introduction.md" "introduction.md (savings bullet)" \
    "(\*\*~)[0-9]+(% fewer input tokens\*\* on real-world data)" "\1${TL_SAVINGS}\2"

update_file "$REPO_ROOT/docs-site/src/introduction.md" "introduction.md (Anthropic accuracy)" \
    "(\| Anthropic accuracy \| )[0-9.]+( \| )[0-9.]+( \| )[0-9.]+( \|)" "\1${ACC_ANTHROPIC_TL}\2${ACC_ANTHROPIC_JSON}\3${ACC_ANTHROPIC_TOON}\4"

update_file "$REPO_ROOT/docs-site/src/introduction.md" "introduction.md (OpenAI accuracy)" \
    "(\| OpenAI accuracy \| )[0-9.]+( \| )[0-9.]+( \| )[0-9.]+( \|)" "\1${ACC_OPENAI_TL}\2${ACC_OPENAI_JSON}\3${ACC_OPENAI_TOON}\4"

update_file "$REPO_ROOT/docs-site/src/introduction.md" "introduction.md (input savings)" \
    "(\| Input token savings \| \*\*-)[0-9]+(%\*\* \| baseline \| \*\*-)[0-9]+(%\*\* \|)" "\1${TL_SAVINGS}\2${TOON_SAVINGS}\3"

# docs-site/src/guides/llm-context.md
update_file "$REPO_ROOT/docs-site/src/guides/llm-context.md" "llm-context.md (savings stat)" \
    "(expect \*\*~)[0-9]+(% fewer data tokens\*\*)" "\1${TL_SAVINGS}\2"

update_file "$REPO_ROOT/docs-site/src/guides/llm-context.md" "llm-context.md (Anthropic accuracy)" \
    "(\| Anthropic accuracy \| )[0-9.]+( \| )[0-9.]+( \| )[0-9.]+( \|)" "\1${ACC_ANTHROPIC_TL}\2${ACC_ANTHROPIC_JSON}\3${ACC_ANTHROPIC_TOON}\4"

update_file "$REPO_ROOT/docs-site/src/guides/llm-context.md" "llm-context.md (OpenAI accuracy)" \
    "(\| OpenAI accuracy \| )[0-9.]+( \| )[0-9.]+( \| )[0-9.]+( \|)" "\1${ACC_OPENAI_TL}\2${ACC_OPENAI_JSON}\3${ACC_OPENAI_TOON}\4"

update_file "$REPO_ROOT/docs-site/src/guides/llm-context.md" "llm-context.md (input savings)" \
    "(\| Input token savings \| \*\*-)[0-9]+(%\*\* \| baseline \| \*\*-)[0-9]+(%\*\* \|)" "\1${TL_SAVINGS}\2${TOON_SAVINGS}\3"

# docs-site/src/internals/accuracy-benchmark.md
update_file "$REPO_ROOT/docs-site/src/internals/accuracy-benchmark.md" "accuracy-benchmark.md (savings headline)" \
    "(\*\*~)[0-9]+(% input token savings\*\* on real-world data)" "\1${TL_SAVINGS}\2"

# Regenerate workflow diagram (picks up version from release.json)
DIAGRAM_SCRIPT="$REPO_ROOT/assets/generate_workflow_diagram.py"
if [[ -f "$DIAGRAM_SCRIPT" ]]; then
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "[DRY RUN] Would regenerate: assets/tealeaf_workflow.png"
    else
        echo "Regenerating workflow diagram..."
        if (cd "$REPO_ROOT" && python "$DIAGRAM_SCRIPT"); then
            echo "Updated: assets/tealeaf_workflow.png"
        else
            echo "Warning: Failed to regenerate workflow diagram" >&2
        fi
    fi
else
    echo "No changes: assets/generate_workflow_diagram.py (not found)"
fi

echo ""
if [[ "$DRY_RUN" == "true" ]]; then
    echo "Dry run complete. No files were modified."
    if [[ "$UPDATES_NEEDED" == "true" ]]; then
        exit 2
    fi
else
    echo "Version sync complete!"
fi
