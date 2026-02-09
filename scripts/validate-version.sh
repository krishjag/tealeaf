#!/bin/bash
#
# Validates that all version references match release.json.
# Exits with error if any file needs updating.
#
# Usage:
#   ./scripts/validate-version.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Run sync in dry-run mode and capture exit code
"$SCRIPT_DIR/sync-version.sh" --dry-run
SYNC_EXIT=$?

# Check exit code (2 = updates needed, 0 = in sync)
if [[ $SYNC_EXIT -eq 2 ]]; then
    echo ""
    echo "ERROR: Version mismatch detected!"
    echo "Please run './scripts/sync-version.sh' locally and commit the changes."
    exit 1
elif [[ $SYNC_EXIT -eq 0 ]]; then
    echo ""
    echo "All versions are in sync."
    exit 0
else
    echo ""
    echo "ERROR: sync-version.sh failed with exit code $SYNC_EXIT"
    exit $SYNC_EXIT
fi
