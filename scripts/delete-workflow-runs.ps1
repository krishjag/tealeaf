#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Delete all GitHub Actions workflow runs for the TeaLeaf repository.

.DESCRIPTION
    This script uses the GitHub CLI (gh) to delete all workflow runs.
    Requires: gh CLI installed and authenticated (gh auth login)

.PARAMETER Confirm
    If specified, prompts for confirmation before deleting.

.PARAMETER DryRun
    If specified, lists runs that would be deleted without actually deleting them.

.PARAMETER Workflow
    Optional workflow name or ID to delete runs from a specific workflow only.

.EXAMPLE
    .\delete-workflow-runs.ps1
    Delete all workflow runs without confirmation.

.EXAMPLE
    .\delete-workflow-runs.ps1 -Confirm
    Prompt for confirmation before deleting.

.EXAMPLE
    .\delete-workflow-runs.ps1 -DryRun
    List all runs without deleting.

.EXAMPLE
    .\delete-workflow-runs.ps1 -Workflow "Rust CLI"
    Delete runs only from the "Rust CLI" workflow.
#>

param(
    [switch]$Confirm,
    [switch]$DryRun,
    [string]$Workflow
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

Write-Host "🔍 Fetching workflow runs from $repo..." -ForegroundColor Cyan

# Build the gh command
$ghArgs = @("run", "list", "--repo", $repo, "--limit", "1000", "--json", "databaseId,name,status,conclusion,createdAt")
if ($Workflow) {
    $ghArgs += @("--workflow", $Workflow)
}

# Get all workflow runs
$runs = gh @ghArgs | ConvertFrom-Json

if ($runs.Count -eq 0) {
    Write-Host "✅ No workflow runs found." -ForegroundColor Green
    exit 0
}

Write-Host "📊 Found $($runs.Count) workflow run(s)" -ForegroundColor Yellow

if ($DryRun) {
    Write-Host "`n🔍 DRY RUN - Would delete the following runs:" -ForegroundColor Magenta
    $runs | ForEach-Object {
        $status = if ($_.conclusion) { $_.conclusion } else { $_.status }
        Write-Host "  - [$status] $($_.name) (ID: $($_.databaseId)) - $($_.createdAt)"
    }
    Write-Host "`n✅ Dry run complete. No runs were deleted." -ForegroundColor Green
    exit 0
}

if ($Confirm) {
    Write-Host "`n⚠️  This will delete $($runs.Count) workflow run(s)." -ForegroundColor Yellow
    $response = Read-Host "Are you sure you want to continue? (yes/no)"
    if ($response -ne "yes") {
        Write-Host "❌ Cancelled." -ForegroundColor Red
        exit 0
    }
}

Write-Host "`n🗑️  Deleting workflow runs..." -ForegroundColor Red

$deleted = 0
$failed = 0

foreach ($run in $runs) {
    $runId = $run.databaseId
    $runName = $run.name

    try {
        gh run delete $runId --repo $repo 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            $deleted++
            Write-Host "  ✓ Deleted: $runName (ID: $runId)" -ForegroundColor Green
        } else {
            $failed++
            Write-Host "  ✗ Failed: $runName (ID: $runId)" -ForegroundColor Red
        }
    }
    catch {
        $failed++
        Write-Host "  ✗ Error deleting $runName (ID: $runId): $_" -ForegroundColor Red
    }
}

Write-Host "`n📊 Summary:" -ForegroundColor Cyan
Write-Host "  ✅ Deleted: $deleted" -ForegroundColor Green
if ($failed -gt 0) {
    Write-Host "  ❌ Failed: $failed" -ForegroundColor Red
}

Write-Host "`n✨ Done!" -ForegroundColor Green
