#requires -Version 5.1
<#
.SYNOPSIS
    Exit gate verifier: runs all 5 exit gates and reports PASS/FAIL.
#>
$ErrorActionPreference = "Continue"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

$root = (Get-Item $PSScriptRoot).Parent.Parent.FullName
Set-Location $root

Write-Host "=== Exit Gate 1/5: cargo test --workspace ===" -ForegroundColor Cyan
cargo test --workspace 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "EXIT GATE: FAIL — cargo test" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 2/5: cargo clippy --workspace -- -D warnings ===" -ForegroundColor Cyan
cargo clippy --workspace -- -D warnings 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "EXIT GATE: FAIL — cargo clippy" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 3/5: run-audit.ps1 ===" -ForegroundColor Cyan
$auditOut = & $PSScriptRoot\run-audit.ps1 2>&1 6>&1
$auditStr = ($auditOut | ForEach-Object { "$_" }) -join "`n"
Write-Host $auditStr
if ($auditStr -match "(\d+) failing") {
    $failingCount = [int]$Matches[1]
    if ($failingCount -gt 0) {
        Write-Host "EXIT GATE: FAIL — audit has $failingCount failing cells" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "EXIT GATE: FAIL — could not parse audit output" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 4/5: diff-ir.ps1 ===" -ForegroundColor Cyan
$diffOut = & $PSScriptRoot\diff-ir.ps1 2>&1 6>&1
$diffStr = ($diffOut | ForEach-Object { "$_" }) -join "`n"
Write-Host $diffStr
if ($diffStr -match "(\d+) unexpected DIFF") {
    $diffCount = [int]$Matches[1]
    if ($diffCount -gt 0) {
        Write-Host "EXIT GATE: FAIL — diff-ir has $diffCount unexpected DIFFs" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "EXIT GATE: FAIL — could not parse diff-ir output" -ForegroundColor Red
    exit 1
}

Write-Host "=== Exit Gate 5/5: audit_regression tests ===" -ForegroundColor Cyan
cargo test -p tangle-cli --test G1_struct_symbol_resolution --test G2_method_param_scope --test G3_member_access_cascade --test G5_doc_drift --test G6_platform_diff --test G7_heading_prefix 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "EXIT GATE: FAIL — audit_regression tests" -ForegroundColor Red
    exit 1
}

Write-Host "EXIT GATE: PASS" -ForegroundColor Green
exit 0
