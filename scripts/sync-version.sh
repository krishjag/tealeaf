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

echo "Release Metadata:"
echo "  Version:    $VERSION"
echo "  Authors:    $AUTHORS"
echo "  License:    $LICENSE"
echo "  Repository: $REPOSITORY"
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

echo ""
if [[ "$DRY_RUN" == "true" ]]; then
    echo "Dry run complete. No files were modified."
    if [[ "$UPDATES_NEEDED" == "true" ]]; then
        exit 2
    fi
else
    echo "Version sync complete!"
fi
