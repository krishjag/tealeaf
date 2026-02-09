# TeaLeaf Scripts

Utility scripts for TeaLeaf project maintenance and automation.

## Version Management

### `sync-version.ps1` / `sync-version.sh`

Synchronizes version, license, and metadata from `release.json` to all project files.

```bash
# Preview changes (dry run)
./scripts/sync-version.ps1 -DryRun
./scripts/sync-version.sh --dry-run

# Apply changes
./scripts/sync-version.ps1
./scripts/sync-version.sh
```

**Updates:**
- `Cargo.toml` (all workspace members)
- `*.csproj` (all .NET projects)
- `README.md`, `CLAUDE.md`, `spec/TEALEAF_SPEC.md`
- `cbindgen.toml`, `docfx.json`
- Documentation site files (`introduction.md`, `comparison-matrix.md`, etc.)

### `validate-version.ps1` / `validate-version.sh`

Validates that all project files have consistent versions matching `release.json`.

```bash
./scripts/validate-version.ps1
./scripts/validate-version.sh
```

**Exits with code 1 if mismatches are found.**

## Testing & Coverage

### `coverage.ps1` / `coverage.sh`

Runs tests with coverage collection for both Rust and .NET.

```bash
# Full coverage (Rust + .NET)
./scripts/coverage.ps1
./scripts/coverage.sh

# Rust only
./scripts/coverage.ps1 -RustOnly
./scripts/coverage.sh --rust-only

# .NET only
./scripts/coverage.ps1 -DotnetOnly
./scripts/coverage.sh --dotnet-only

# CI mode (lcov + cobertura, no HTML)
./scripts/coverage.ps1 -CI
./scripts/coverage.sh --ci
```

**Generates:**
- Rust: HTML report in `coverage/` + `coverage.lcov`
- .NET: HTML report in `bindings/dotnet/coverage/` + `coverage.cobertura.xml`

**Requirements:**
- Rust: `cargo-llvm-cov` (`cargo install cargo-llvm-cov`)
- .NET: `coverlet.collector` package (included in test projects)

## GitHub Actions & Deployments

### `delete-workflow-runs.ps1` / `delete-workflow-runs.sh`

Deletes GitHub Actions workflow runs. Useful for cleanup or resetting workflow history.

**Requirements:**
- GitHub CLI (`gh`) installed and authenticated (`gh auth login`)
- Token with `workflow` scope

```bash
# Delete all workflow runs (no confirmation)
./scripts/delete-workflow-runs.ps1
./scripts/delete-workflow-runs.sh

# Prompt for confirmation before deleting
./scripts/delete-workflow-runs.ps1 -Confirm
./scripts/delete-workflow-runs.sh --confirm

# Dry run - list what would be deleted without deleting
./scripts/delete-workflow-runs.ps1 -DryRun
./scripts/delete-workflow-runs.sh --dry-run

# Delete runs from a specific workflow only
./scripts/delete-workflow-runs.ps1 -Workflow "Rust CLI"
./scripts/delete-workflow-runs.sh --workflow "Rust CLI"
```

### `delete-deployments.ps1` / `delete-deployments.sh`

Deletes GitHub deployments. Useful for cleaning up deployment history.

**Requirements:**
- GitHub CLI (`gh`) installed and authenticated (`gh auth login`)
- Token with `repo` and `repo_deployment` scopes (or "Read and Write access to deployments")

```bash
# Delete all deployments (no confirmation)
./scripts/delete-deployments.ps1
./scripts/delete-deployments.sh

# Prompt for confirmation before deleting
./scripts/delete-deployments.ps1 -Confirm
./scripts/delete-deployments.sh --confirm

# Dry run - list what would be deleted without deleting
./scripts/delete-deployments.ps1 -DryRun
./scripts/delete-deployments.sh --dry-run

# Delete deployments from a specific environment only
./scripts/delete-deployments.ps1 -Environment "github-pages"
./scripts/delete-deployments.sh --environment "github-pages"
```

## Platform Notes

- **PowerShell scripts** (`.ps1`): Cross-platform (PowerShell Core 7+)
- **Bash scripts** (`.sh`): Linux, macOS, Git Bash on Windows
- Both script variants have identical functionality

## CI/CD Integration

These scripts are used in GitHub Actions workflows:

- **Version sync** is required before releases (manual step)
- **Version validation** runs on every PR to ensure consistency
- **Coverage** runs on push/PR and uploads to Codecov
- **Workflow cleanup** is manual (not automated in CI)
