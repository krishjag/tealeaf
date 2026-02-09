#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Delete GitHub Actions caches for the TeaLeaf repository.

.DESCRIPTION
    This script uses the GitHub CLI (gh) to delete Actions caches.
    Requires: gh CLI installed and authenticated (gh auth login)

.PARAMETER Confirm
    If specified, prompts for confirmation before deleting.

.PARAMETER DryRun
    If specified, lists caches that would be deleted without actually deleting them.

.PARAMETER Key
    Optional cache key prefix to filter. Only caches whose key starts with this value are deleted.

.PARAMETER Branch
    Optional branch name (ref) to filter caches by.

.EXAMPLE
    .\delete-caches.ps1
    Delete all caches without confirmation.

.EXAMPLE
    .\delete-caches.ps1 -Confirm
    Prompt for confirmation before deleting.

.EXAMPLE
    .\delete-caches.ps1 -DryRun
    List all caches without deleting.

.EXAMPLE
    .\delete-caches.ps1 -Key "cargo-"
    Delete only caches whose key starts with "cargo-".

.EXAMPLE
    .\delete-caches.ps1 -Branch "refs/pull/42/merge"
    Delete caches from a specific branch/PR ref.
#>

param(
    [switch]$Confirm,
    [switch]$DryRun,
    [string]$Key,
    [string]$Branch
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

Write-Host "Fetching caches from $repo..." -ForegroundColor Cyan

# Build the API query
$apiPath = "repos/$repo/actions/caches?per_page=100"
if ($Key) {
    $apiPath += "&key=$Key"
}
if ($Branch) {
    $apiPath += "&ref=$Branch"
}

# Paginate through all caches
$allCaches = @()
$page = 1

do {
    $pagedPath = "$apiPath&page=$page"
    $response = gh api $pagedPath 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to fetch caches: $response"
        exit 1
    }

    $data = $response | ConvertFrom-Json
    $caches = $data.actions_caches

    if ($caches.Count -eq 0) { break }

    $allCaches += $caches
    $page++
} while ($caches.Count -eq 100)

if ($allCaches.Count -eq 0) {
    Write-Host "No caches found." -ForegroundColor Green
    exit 0
}

# Calculate total size
$totalSizeMB = [math]::Round(($allCaches | Measure-Object -Property size_in_bytes -Sum).Sum / 1MB, 1)

Write-Host "Found $($allCaches.Count) cache(s) ($totalSizeMB MB total)" -ForegroundColor Yellow

if ($DryRun) {
    Write-Host "`nDRY RUN - Would delete the following caches:" -ForegroundColor Magenta
    $allCaches | ForEach-Object {
        $sizeMB = [math]::Round($_.size_in_bytes / 1MB, 1)
        $age = if ($_.last_accessed_at) {
            $days = [math]::Round(((Get-Date) - [datetime]$_.last_accessed_at).TotalDays, 0)
            "${days}d ago"
        } else { "unknown" }
        Write-Host "  - [$sizeMB MB] $($_.key) (ref: $($_.ref), last used: $age)"
    }
    Write-Host "`nDry run complete. No caches were deleted." -ForegroundColor Green
    exit 0
}

if ($Confirm) {
    Write-Host "`nThis will delete $($allCaches.Count) cache(s) ($totalSizeMB MB)." -ForegroundColor Yellow
    $response = Read-Host "Are you sure you want to continue? (yes/no)"
    if ($response -ne "yes") {
        Write-Host "Cancelled." -ForegroundColor Red
        exit 0
    }
}

Write-Host "`nDeleting caches..." -ForegroundColor Red

$deleted = 0
$failed = 0
$freedMB = 0

foreach ($cache in $allCaches) {
    $cacheId = $cache.id
    $cacheKey = $cache.key
    $sizeMB = [math]::Round($cache.size_in_bytes / 1MB, 1)

    try {
        gh api --method DELETE "repos/$repo/actions/caches/$cacheId" 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            $deleted++
            $freedMB += $sizeMB
            Write-Host "  Deleted: $cacheKey ($sizeMB MB)" -ForegroundColor Green
        } else {
            $failed++
            Write-Host "  Failed: $cacheKey" -ForegroundColor Red
        }
    }
    catch {
        $failed++
        Write-Host "  Error deleting $cacheKey : $_" -ForegroundColor Red
    }
}

Write-Host "`nSummary:" -ForegroundColor Cyan
Write-Host "  Deleted: $deleted ($freedMB MB freed)" -ForegroundColor Green
if ($failed -gt 0) {
    Write-Host "  Failed: $failed" -ForegroundColor Red
}

Write-Host "`nDone!" -ForegroundColor Green
