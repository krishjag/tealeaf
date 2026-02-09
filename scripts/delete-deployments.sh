#!/usr/bin/env bash
#
# Delete all GitHub deployments for the TeaLeaf repository.
# Requires: gh CLI installed and authenticated (gh auth login)
# Requires: Token with 'repo' and 'repo_deployment' scopes
#
# Usage:
#   ./delete-deployments.sh              # Delete all deployments
#   ./delete-deployments.sh --confirm    # Prompt for confirmation
#   ./delete-deployments.sh --dry-run    # List deployments without deleting
#   ./delete-deployments.sh --environment "github-pages"  # Delete from specific environment

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
ENVIRONMENT=""
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
        --environment)
            ENVIRONMENT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--confirm] [--dry-run] [--environment <name>]"
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

echo -e "${CYAN}🔍 Fetching deployments from $REPO...${NC}"

# Build the gh API command
if [[ -n "$ENVIRONMENT" ]]; then
    API_ENDPOINT="repos/$REPO/deployments?environment=$ENVIRONMENT"
else
    API_ENDPOINT="repos/$REPO/deployments"
fi

# Get all deployments
DEPLOYMENTS_JSON=$(gh api "$API_ENDPOINT" --paginate -q '.')
DEPLOYMENT_COUNT=$(echo "$DEPLOYMENTS_JSON" | jq '. | length')

if [[ "$DEPLOYMENT_COUNT" -eq 0 ]]; then
    echo -e "${GREEN}✅ No deployments found.${NC}"
    exit 0
fi

echo -e "${YELLOW}📊 Found $DEPLOYMENT_COUNT deployment(s)${NC}"

if [[ "$DRY_RUN" = true ]]; then
    echo -e "\n${MAGENTA}🔍 DRY RUN - Would delete the following deployments:${NC}"
    echo "$DEPLOYMENTS_JSON" | jq -r '.[] | "  - [\(.environment)] Deployment ID: \(.id) - Created: \(.created_at)"'
    echo -e "\n${GREEN}✅ Dry run complete. No deployments were deleted.${NC}"
    exit 0
fi

if [[ "$CONFIRM" = true ]]; then
    echo -e "\n${YELLOW}⚠️  This will delete $DEPLOYMENT_COUNT deployment(s).${NC}"
    read -p "Are you sure you want to continue? (yes/no): " response
    if [[ "$response" != "yes" ]]; then
        echo -e "${RED}❌ Cancelled.${NC}"
        exit 0
    fi
fi

echo -e "\n${RED}🗑️  Deleting deployments...${NC}"

deleted=0
failed=0

while IFS= read -r deployment_id; do
    env=$(echo "$DEPLOYMENTS_JSON" | jq -r ".[] | select(.id == $deployment_id) | .environment")

    # Set deployment status to inactive first
    if ! gh api -X POST "repos/$REPO/deployments/$deployment_id/statuses" -f state=inactive &> /dev/null; then
        echo -e "${YELLOW}  ⚠ Warning: Could not set deployment $deployment_id to inactive${NC}"
    fi

    # Delete the deployment
    if gh api -X DELETE "repos/$REPO/deployments/$deployment_id" &> /dev/null; then
        ((deleted++))
        echo -e "${GREEN}  ✓ Deleted: $env (ID: $deployment_id)${NC}"
    else
        ((failed++))
        echo -e "${RED}  ✗ Failed: $env (ID: $deployment_id)${NC}"
    fi
done < <(echo "$DEPLOYMENTS_JSON" | jq -r '.[].id')

echo -e "\n${CYAN}📊 Summary:${NC}"
echo -e "${GREEN}  ✅ Deleted: $deleted${NC}"
if [[ $failed -gt 0 ]]; then
    echo -e "${RED}  ❌ Failed: $failed${NC}"
fi

echo -e "\n${GREEN}✨ Done!${NC}"
