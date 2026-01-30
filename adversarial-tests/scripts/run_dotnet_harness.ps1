param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\.."))
)

$dotnetRoot = Join-Path $RepoRoot "bindings\dotnet"
$harnessProj = Join-Path $RepoRoot "adversarial-tests\dotnet-harness\AdversarialHarness.csproj"
$config = "Release"

function Get-HostRid {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
        switch ($arch) {
            "X64" { return "win-x64" }
            "X86" { return "win-x86" }
            "Arm64" { return "win-arm64" }
            default { return "win-x64" }
        }
    } elseif ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Linux)) {
        switch ($arch) {
            "X64" { return "linux-x64" }
            "Arm64" { return "linux-arm64" }
            default { return "linux-x64" }
        }
    } elseif ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::OSX)) {
        switch ($arch) {
            "X64" { return "osx-x64" }
            "Arm64" { return "osx-arm64" }
            default { return "osx-x64" }
        }
    }
    return "win-x64"
}
Push-Location $dotnetRoot
try {
    & .\build.ps1 -Configuration $config
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}

& dotnet build $harnessProj -c $config
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$rid = Get-HostRid
$runtimesDir = Join-Path $dotnetRoot "TeaLeaf\runtimes"
$harnessBin8 = Join-Path $RepoRoot "adversarial-tests\dotnet-harness\bin\$config\net8.0"
$harnessBin10 = Join-Path $RepoRoot "adversarial-tests\dotnet-harness\bin\$config\net10.0"

switch -Regex ($rid) {
    "^win-" { $libName = "tealeaf_ffi.dll" }
    "^linux-" { $libName = "libtealeaf_ffi.so" }
    "^osx-" { $libName = "libtealeaf_ffi.dylib" }
    default { $libName = "tealeaf_ffi.dll" }
}

$nativeLib = Join-Path $runtimesDir "$rid\native\$libName"
if (-not (Test-Path $nativeLib)) {
    $nativeLib = Get-ChildItem -Path $runtimesDir -Recurse -Filter $libName |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($nativeLib) {
        $nativeLib = $nativeLib.FullName
    }
}

if (Test-Path $nativeLib) {
    if (Test-Path $harnessBin8) {
        Copy-Item $nativeLib $harnessBin8 -Force
    }
    if (Test-Path $harnessBin10) {
        Copy-Item $nativeLib $harnessBin10 -Force
    }
}

& dotnet run -c $config --no-build --project $harnessProj -- --root $RepoRoot
exit $LASTEXITCODE
