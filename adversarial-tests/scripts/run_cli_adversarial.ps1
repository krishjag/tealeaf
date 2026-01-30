param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\.."))
)

$results = Join-Path $RepoRoot "adversarial-tests\results\cli"
$inputs = Join-Path $RepoRoot "adversarial-tests\inputs"
$examples = Join-Path $RepoRoot "examples"

New-Item -ItemType Directory -Force -Path $results | Out-Null

$failures = @()

function Run-Case([string]$Name, [string[]]$Args, [bool]$ExpectSuccess) {
    $log = Join-Path $results ("{0}.log" -f $Name)
    $cmdArgs = @("run", "-p", "tealeaf-core", "--") + $Args
    & cargo @cmdArgs 2>&1 | Tee-Object -FilePath $log | Out-Null
    $exit = $LASTEXITCODE
    $ok = ($exit -eq 0)
    if ($ok -ne $ExpectSuccess) {
        $failures += "${Name} (exit=$exit)"
    }
}

# Build once to reduce repeated compile time
& cargo build -p tealeaf-core | Out-Null

# Valid baseline
Run-Case "validate_retail_orders" @("validate", (Join-Path $examples "retail_orders.tl")) $true
Run-Case "compile_retail_orders" @("compile", (Join-Path $examples "retail_orders.tl"), "-o", (Join-Path $results "retail_orders.tlbx")) $true
Run-Case "decompile_retail_orders" @("decompile", (Join-Path $results "retail_orders.tlbx"), "-o", (Join-Path $results "retail_orders_decompiled.tl")) $true
Run-Case "to_json_retail_orders" @("to-json", (Join-Path $examples "retail_orders.tl"), "-o", (Join-Path $results "retail_orders.json")) $true
Run-Case "json_to_tlbx_retail_orders" @("json-to-tlbx", (Join-Path $examples "retail_orders.json"), "-o", (Join-Path $results "retail_orders_from_json.tlbx")) $true

# Adversarial TL inputs
$badTl = @(
    "bad_unclosed_string.tl",
    "bad_missing_colon.tl",
    "bad_invalid_escape.tl",
    "bad_number_overflow.tl",
    "bad_table_wrong_arity.tl",
    "bad_schema_unclosed.tl",
    "bad_unicode_escape_short.tl",
    "bad_unicode_escape_invalid_hex.tl",
    "bad_unicode_escape_surrogate.tl",
    "bad_unterminated_multiline.tl",
    "invalid_utf8.tl"
)
foreach ($file in $badTl) {
    $name = [IO.Path]::GetFileNameWithoutExtension($file)
    Run-Case "validate_${name}" @("validate", (Join-Path $inputs "tl\$file")) $false
}

# Deep nesting should parse
Run-Case "validate_deep_nesting" @("validate", (Join-Path $inputs "tl\deep_nesting.tl")) $true

# Adversarial JSON inputs
Run-Case "from_json_invalid_trailing" @("from-json", (Join-Path $inputs "json\invalid_json_trailing.json"), "-o", (Join-Path $results "invalid_json_trailing.tl")) $false
Run-Case "from_json_invalid_unclosed" @("from-json", (Join-Path $inputs "json\invalid_json_unclosed.json"), "-o", (Join-Path $results "invalid_json_unclosed.tl")) $false
Run-Case "from_json_large_number" @("from-json", (Join-Path $inputs "json\large_number.json"), "-o", (Join-Path $results "large_number.tl")) $true
Run-Case "from_json_deep_array" @("from-json", (Join-Path $inputs "json\deep_array.json"), "-o", (Join-Path $results "deep_array.tl")) $true
Run-Case "from_json_root_array" @("from-json", (Join-Path $inputs "json\root_array.json"), "-o", (Join-Path $results "root_array.tl")) $true
Run-Case "from_json_empty_object" @("from-json", (Join-Path $inputs "json\empty_object.json"), "-o", (Join-Path $results "empty_object.tl")) $true

# Adversarial TLBX inputs
Run-Case "info_bad_magic" @("info", (Join-Path $inputs "tlbx\bad_magic.tlbx")) $false
Run-Case "info_truncated_header" @("info", (Join-Path $inputs "tlbx\truncated_header.tlbx")) $false
Run-Case "info_bad_version" @("info", (Join-Path $inputs "tlbx\bad_version.tlbx")) $false
Run-Case "tlbx_to_json_random_garbage" @("tlbx-to-json", (Join-Path $inputs "tlbx\random_garbage.tlbx"), "-o", (Join-Path $results "random_garbage.json")) $false

if ($failures.Count -gt 0) {
    Write-Error "CLI adversarial failures:`n$($failures -join "`n")"
    exit 1
}

Write-Host "CLI adversarial tests passed."
