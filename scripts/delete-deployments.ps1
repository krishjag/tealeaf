#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Delete all GitHub deployments for the TeaLeaf repository.

.DESCRIPTION
    This script uses the GitHub CLI (gh) to delete all deployments.
    Requires: gh CLI installed and authenticated (gh auth login)
    Requires: Token with 'repo' and 'repo_deployment' scopes

.PARAMETER Confirm
    If specified, prompts for confirmation before deleting.

.PARAMETER DryRun
    If specified, lists deployments that would be deleted without actually deleting them.

.PARAMETER Environment
    Optional environment name to delete deployments from a specific environment only.

.EXAMPLE
    .\delete-deployments.ps1
    Delete all deployments without confirmation.

.EXAMPLE
    .\delete-deployments.ps1 -Confirm
    Prompt for confirmation before deleting.

.EXAMPLE
    .\delete-deployments.ps1 -DryRun
    List all deployments without deleting.

.EXAMPLE
    .\delete-deployments.ps1 -Environment "github-pages"
    Delete deployments only from the "github-pages" environment.
#>

param(
    [switch]$Confirm,
    [switch]$DryRun,
    [string]$Environment
)

# Check if gh CLI is installed
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Write-Error "GitHub CLI (gh) is not installed. Install from: https://cli.github.com/"
    exit 1
}

# Check if authenticated
$authStatus = gh auth status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "Not authenticated with GitHub CLI. Run: gh auth login"
    exit 1
}

# Get repository info
$repo = "krishjag/tealeaf"

Write-Host "🔍 Fetching deployments from $repo..." -ForegroundColor Cyan

# Build the gh API command to get deployments
$apiArgs = @("api", "repos/$repo/deployments", "--paginate", "-q", ".")
if ($Environment) {
    $apiArgs = @("api", "repos/$repo/deployments?environment=$Environment", "--paginate", "-q", ".")
}

# Get all deployments
try {
    $deploymentsJson = gh @apiArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to fetch deployments: $deploymentsJson"
        exit 1
    }
    $deployments = $deploymentsJson | ConvertFrom-Json
}
catch {
    Write-Error "Failed to parse deployments: $_"
    exit 1
}

if ($deployments.Count -eq 0) {
    Write-Host "✅ No deployments found." -ForegroundColor Green
    exit 0
}

Write-Host "📊 Found $($deployments.Count) deployment(s)" -ForegroundColor Yellow

if ($DryRun) {
    Write-Host "`n🔍 DRY RUN - Would delete the following deployments:" -ForegroundColor Magenta
    $deployments | ForEach-Object {
        Write-Host "  - [$($_.environment)] Deployment ID: $($_.id) - Created: $($_.created_at)"
    }
    Write-Host "`n✅ Dry run complete. No deployments were deleted." -ForegroundColor Green
    exit 0
}

if ($Confirm) {
    Write-Host "`n⚠️  This will delete $($deployments.Count) deployment(s)." -ForegroundColor Yellow
    $response = Read-Host "Are you sure you want to continue? (yes/no)"
    if ($response -ne "yes") {
        Write-Host "❌ Cancelled." -ForegroundColor Red
        exit 0
    }
}

Write-Host "`n🗑️  Deleting deployments..." -ForegroundColor Red

$deleted = 0
$failed = 0

foreach ($deployment in $deployments) {
    $deploymentId = $deployment.id
    $env = $deployment.environment

    try {
        # Set deployment status to inactive first
        gh api -X POST "repos/$repo/deployments/$deploymentId/statuses" `
            -f state=inactive 2>&1 | Out-Null

        if ($LASTEXITCODE -ne 0) {
            Write-Host "  ⚠ Warning: Could not set deployment $deploymentId to inactive" -ForegroundColor Yellow
        }

        # Delete the deployment
        gh api -X DELETE "repos/$repo/deployments/$deploymentId" 2>&1 | Out-Null

        if ($LASTEXITCODE -eq 0) {
            $deleted++
            Write-Host "  ✓ Deleted: $env (ID: $deploymentId)" -ForegroundColor Green
        } else {
            $failed++
            Write-Host "  ✗ Failed: $env (ID: $deploymentId)" -ForegroundColor Red
        }
    }
    catch {
        $failed++
        Write-Host "  ✗ Error deleting $env (ID: $deploymentId): $_" -ForegroundColor Red
    }
}

Write-Host "`n📊 Summary:" -ForegroundColor Cyan
Write-Host "  ✅ Deleted: $deleted" -ForegroundColor Green
if ($failed -gt 0) {
    Write-Host "  ❌ Failed: $failed" -ForegroundColor Red
}

Write-Host "`n✨ Done!" -ForegroundColor Green
