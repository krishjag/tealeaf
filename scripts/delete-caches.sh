#!/usr/bin/env bash
#
# Delete GitHub Actions caches for the TeaLeaf repository.
# Requires: gh CLI installed and authenticated (gh auth login)
#
# Usage:
#   ./delete-caches.sh                     # Delete all caches
#   ./delete-caches.sh --confirm           # Prompt for confirmation
#   ./delete-caches.sh --dry-run           # List caches without deleting
#   ./delete-caches.sh --key "cargo-"      # Delete caches matching key prefix
#   ./delete-caches.sh --branch "refs/pull/42/merge"  # Delete caches from specific ref

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
KEY=""
BRANCH=""
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
        --key)
            KEY="$2"
            shift 2
            ;;
        --branch)
            BRANCH="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--confirm] [--dry-run] [--key <prefix>] [--branch <ref>]"
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

echo -e "${CYAN}Fetching caches from $REPO...${NC}"

# Build the API query
API_PATH="repos/$REPO/actions/caches?per_page=100"
if [[ -n "$KEY" ]]; then
    API_PATH="${API_PATH}&key=$KEY"
fi
if [[ -n "$BRANCH" ]]; then
    API_PATH="${API_PATH}&ref=$BRANCH"
fi

# Paginate through all caches
ALL_CACHES="[]"
page=1

while true; do
    RESPONSE=$(gh api "${API_PATH}&page=$page" 2>&1) || {
        echo -e "${RED}Error: Failed to fetch caches: $RESPONSE${NC}"
        exit 1
    }

    CACHES=$(echo "$RESPONSE" | jq '.actions_caches')
    COUNT=$(echo "$CACHES" | jq 'length')

    if [[ "$COUNT" -eq 0 ]]; then
        break
    fi

    ALL_CACHES=$(echo "$ALL_CACHES $CACHES" | jq -s '.[0] + .[1]')
    ((page++))

    if [[ "$COUNT" -lt 100 ]]; then
        break
    fi
done

TOTAL_COUNT=$(echo "$ALL_CACHES" | jq 'length')

if [[ "$TOTAL_COUNT" -eq 0 ]]; then
    echo -e "${GREEN}No caches found.${NC}"
    exit 0
fi

# Calculate total size
TOTAL_SIZE_MB=$(echo "$ALL_CACHES" | jq '[.[].size_in_bytes] | add / 1048576 | . * 10 | floor / 10')

echo -e "${YELLOW}Found $TOTAL_COUNT cache(s) (${TOTAL_SIZE_MB} MB total)${NC}"

if [[ "$DRY_RUN" = true ]]; then
    echo -e "\n${MAGENTA}DRY RUN - Would delete the following caches:${NC}"
    echo "$ALL_CACHES" | jq -r '.[] | "  - [\(.size_in_bytes / 1048576 | . * 10 | floor / 10) MB] \(.key) (ref: \(.ref), last used: \(.last_accessed_at // "unknown"))"'
    echo -e "\n${GREEN}Dry run complete. No caches were deleted.${NC}"
    exit 0
fi

if [[ "$CONFIRM" = true ]]; then
    echo -e "\n${YELLOW}This will delete $TOTAL_COUNT cache(s) (${TOTAL_SIZE_MB} MB).${NC}"
    read -p "Are you sure you want to continue? (yes/no): " response
    if [[ "$response" != "yes" ]]; then
        echo -e "${RED}Cancelled.${NC}"
        exit 0
    fi
fi

echo -e "\n${RED}Deleting caches...${NC}"

deleted=0
failed=0
freed_mb=0

while IFS=$'\t' read -r cache_id cache_key size_mb; do
    if gh api --method DELETE "repos/$REPO/actions/caches/$cache_id" &> /dev/null; then
        ((deleted++))
        freed_mb=$(echo "$freed_mb + $size_mb" | bc)
        echo -e "${GREEN}  Deleted: $cache_key ($size_mb MB)${NC}"
    else
        ((failed++)) || true
        echo -e "${RED}  Failed: $cache_key${NC}"
    fi
done < <(echo "$ALL_CACHES" | jq -r '.[] | [.id, .key, (.size_in_bytes / 1048576 | . * 10 | floor / 10)] | @tsv')

echo -e "\n${CYAN}Summary:${NC}"
echo -e "${GREEN}  Deleted: $deleted ($freed_mb MB freed)${NC}"
if [[ $failed -gt 0 ]]; then
    echo -e "${RED}  Failed: $failed${NC}"
fi

echo -e "\n${GREEN}Done!${NC}"
