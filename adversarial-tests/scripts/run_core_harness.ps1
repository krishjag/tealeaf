param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\.."))
)

Push-Location (Join-Path $RepoRoot "adversarial-tests\core-harness")
try {
    & cargo test
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
