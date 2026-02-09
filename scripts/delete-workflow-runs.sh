#!/usr/bin/env bash
#
# Delete all GitHub Actions workflow runs for the TeaLeaf repository.
# Requires: gh CLI installed and authenticated (gh auth login)
#
# Usage:
#   ./delete-workflow-runs.sh              # Delete all runs
#   ./delete-workflow-runs.sh --confirm    # Prompt for confirmation
#   ./delete-workflow-runs.sh --dry-run    # List runs without deleting
#   ./delete-workflow-runs.sh --workflow "Rust CLI"  # Delete runs from specific workflow

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Default values
CONFIRM=false
DRY_RUN=false
WORKFLOW=""
REPO="krishjag/tealeaf"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --confirm)
            CONFIRM=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --workflow)
            WORKFLOW="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--confirm] [--dry-run] [--workflow <name>]"
            exit 1
            ;;
    esac
done

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo -e "${RED}Error: GitHub CLI (gh) is not installed.${NC}"
    echo "Install from: https://cli.github.com/"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo -e "${RED}Error: Not authenticated with GitHub CLI.${NC}"
    echo "Run: gh auth login"
    exit 1
fi

echo -e "${CYAN}🔍 Fetching workflow runs from $REPO...${NC}"

# Build the gh command
GH_ARGS=("run" "list" "--repo" "$REPO" "--limit" "1000" "--json" "databaseId,name,status,conclusion,createdAt")
if [[ -n "$WORKFLOW" ]]; then
    GH_ARGS+=("--workflow" "$WORKFLOW")
fi

# Get all workflow runs
RUNS_JSON=$(gh "${GH_ARGS[@]}")
RUN_COUNT=$(echo "$RUNS_JSON" | jq '. | length')

if [[ "$RUN_COUNT" -eq 0 ]]; then
    echo -e "${GREEN}✅ No workflow runs found.${NC}"
    exit 0
fi

echo -e "${YELLOW}📊 Found $RUN_COUNT workflow run(s)${NC}"

if [[ "$DRY_RUN" = true ]]; then
    echo -e "\n${MAGENTA}🔍 DRY RUN - Would delete the following runs:${NC}"
    echo "$RUNS_JSON" | jq -r '.[] | "  - [\(.conclusion // .status)] \(.name) (ID: \(.databaseId)) - \(.createdAt)"'
    echo -e "\n${GREEN}✅ Dry run complete. No runs were deleted.${NC}"
    exit 0
fi

if [[ "$CONFIRM" = true ]]; then
    echo -e "\n${YELLOW}⚠️  This will delete $RUN_COUNT workflow run(s).${NC}"
    read -p "Are you sure you want to continue? (yes/no): " response
    if [[ "$response" != "yes" ]]; then
        echo -e "${RED}❌ Cancelled.${NC}"
        exit 0
    fi
fi

echo -e "\n${RED}🗑️  Deleting workflow runs...${NC}"

deleted=0
failed=0

while IFS= read -r run_id; do
    run_name=$(echo "$RUNS_JSON" | jq -r ".[] | select(.databaseId == $run_id) | .name")

    if gh run delete "$run_id" --repo "$REPO" &> /dev/null; then
        ((deleted++))
        echo -e "${GREEN}  ✓ Deleted: $run_name (ID: $run_id)${NC}"
    else
        ((failed++))
        echo -e "${RED}  ✗ Failed: $run_name (ID: $run_id)${NC}"
    fi
done < <(echo "$RUNS_JSON" | jq -r '.[].databaseId')

echo -e "\n${CYAN}📊 Summary:${NC}"
echo -e "${GREEN}  ✅ Deleted: $deleted${NC}"
if [[ $failed -gt 0 ]]; then
    echo -e "${RED}  ❌ Failed: $failed${NC}"
fi

echo -e "\n${GREEN}✨ Done!${NC}"
