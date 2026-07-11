#requires -Version 5.1
<#
.SYNOPSIS
    Differential IR testing: TS reference vs Rust compiler.

.DESCRIPTION
    For each .tangle.md fixture, emits IR from both the TypeScript reference
    compiler (node reference/dist/src/cli/main.js run <fixture> --emit-ir) and
    the Rust compiler (cargo run -- build <fixture> --emit-ir), then compares
    the two IR JSONs with ir-diff.exe (semantic comparison: source spans
    stripped, object keys sorted).

    Exit code 0 = all MATCH or SKIPPED, 1 = at least one DIFF.
#>
$ErrorActionPreference = "Continue"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

$root = (Get-Item $PSScriptRoot).Parent.Parent.FullName
Set-Location $root

# --- Build ir-diff if needed -------------------------------------------------
$irDiffBin = Join-Path $PSScriptRoot "ir-diff\target\release\ir-diff.exe"
if (-not (Test-Path $irDiffBin)) {
    Write-Host "Building ir-diff..."
    cargo build --release --manifest-path (Join-Path $PSScriptRoot "ir-diff\Cargo.toml") | Out-Null
}

# --- Ensure TS reference is built -------------------------------------------
# node_modules must be installed before `npm run build` will succeed.
if (-not (Test-Path (Join-Path $root "reference\node_modules"))) {
    Write-Host "Installing TS reference dependencies..."
    Push-Location (Join-Path $root "reference")
    npm install 2>&1 | Out-Null
    Pop-Location
}

$tsEntry = Join-Path $root "reference\dist\src\cli\main.js"
if (-not (Test-Path $tsEntry)) {
    Write-Host "Building TS reference..."
    Push-Location (Join-Path $root "reference")
    npm run build 2>&1 | Out-Null
    Pop-Location
}

# --- Collect fixtures --------------------------------------------------------
$fixtures = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName

$workDir = Join-Path $env:TEMP "tangle-diff-ir"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

$matchCount = 0
$diffCount = 0
$skipCount = 0

foreach ($fixture in $fixtures) {
    $name = [System.IO.Path]::GetFileNameWithoutExtension($fixture)
    $tsIr = Join-Path $workDir "$name.ts.json"
    $rsIr = Join-Path $workDir "$name.rs.json"

    # --- TS reference --------------------------------------------------------
    $tsErr = Join-Path $workDir "$name.ts.err"
    $tsProc = Start-Process -FilePath "node" -ArgumentList @($tsEntry, "run", $fixture, "--emit-ir") -NoNewWindow -PassThru -Wait -RedirectStandardOutput $tsIr -RedirectStandardError $tsErr
    if ($tsProc.ExitCode -ne 0) {
        Write-Host "[SKIPPED] $name - TS reference failed"
        $skipCount++
        continue
    }

    # --- Rust compiler -------------------------------------------------------
    $rsErr = Join-Path $workDir "$name.rs.err"
    $rsProc = Start-Process -FilePath "cargo" -ArgumentList @("run","--quiet","--","build",$fixture,"--emit-ir") -NoNewWindow -PassThru -Wait -RedirectStandardOutput $rsIr -RedirectStandardError $rsErr
    if ($rsProc.ExitCode -ne 0 -and -not (Test-Path $rsIr)) {
        Write-Host "[DIFF] $name - Rust failed to emit IR"
        $diffCount++
        continue
    }

    # --- Compare -------------------------------------------------------------
    $cmpOut = Join-Path $workDir "$name.cmp.out"
    $cmpErr = Join-Path $workDir "$name.cmp.err"
    $cmpProc = Start-Process -FilePath $irDiffBin -ArgumentList @($tsIr, $rsIr) -NoNewWindow -PassThru -Wait -RedirectStandardOutput $cmpOut -RedirectStandardError $cmpErr
    if (Test-Path $cmpOut) {
        $cmpOutText = Get-Content $cmpOut -Raw
    } else {
        $cmpOutText = ""
    }
    if ($cmpOutText -match "MATCH") {
        Write-Host "[MATCH] $name"
        $matchCount++
    } else {
        Write-Host "[DIFF] $name"
        $diffCount++
    }
}

Write-Host ""
Write-Host "Diff-IR complete: $matchCount MATCH, $diffCount DIFF, $skipCount SKIPPED"
exit $(if ($diffCount -gt 0) { 1 } else { 0 })
