<#
.SYNOPSIS
    Validates that all version references match release.json.

.DESCRIPTION
    Runs sync-version in dry-run mode and fails if any files need updating.
    Use this in CI to ensure versions are committed correctly.

.EXAMPLE
    ./scripts/validate-version.ps1
#>

$ScriptDir = $PSScriptRoot

# Run sync in dry-run mode
& "$ScriptDir/sync-version.ps1" -DryRun
$syncExitCode = $LASTEXITCODE

# Check exit code (2 = updates needed, 0 = in sync)
if ($syncExitCode -eq 2) {
    Write-Host ""
    Write-Host "ERROR: Version mismatch detected!" -ForegroundColor Red
    Write-Host "Please run './scripts/sync-version.ps1' locally and commit the changes." -ForegroundColor Red
    exit 1
} elseif ($syncExitCode -eq 0 -or $null -eq $syncExitCode) {
    Write-Host ""
    Write-Host "All versions are in sync." -ForegroundColor Green
    exit 0
} else {
    Write-Host ""
    Write-Host "ERROR: sync-version.ps1 failed with exit code $syncExitCode" -ForegroundColor Red
    exit $syncExitCode
}
