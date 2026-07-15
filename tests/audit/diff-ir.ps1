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

    Fixtures with known IR schema divergence (F-007~F-012, deferred to v0.3.0)
    are listed in $KnownDiffs and reported as KNOWN_DIFF rather than failing
    the gate. When TS emits an empty IR (F-010: rule-based fixtures not yet
    implemented in the TS reference), the fixture is SKIPPED.

    Exit code 0 = 0 unexpected DIFF, 1 = at least one unexpected DIFF.
#>
$ErrorActionPreference = "Continue"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

$root = (Get-Item $PSScriptRoot).Parent.Parent.FullName
Set-Location $root

# --- Known DIFF allowlist ----------------------------------------------------
# F-007~F-012 closed in Phase 4 via ir-diff normalization (lift_functions,
# build_id_map, null guard strip, label normalize). 3 of 4 fixtures now MATCH.
# payment.tangle remains KNOWN_DIFF due to structural difference: Rust IR has
# both top-level merged nodes AND functions array (dual-entry), while TS IR
# uses a single shared entry. This is a deeper IR generation difference that
# needs separate investigation (future phase).
$KnownDiffs = @(
    "payment.tangle"       # structural: Rust dual-entry (top-level + functions) vs TS shared entry
)

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
$knownDiffCount = 0
$unexpectedDiffCount = 0
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
        $unexpectedDiffCount++
        continue
    }

    # --- Check for empty TS IR (F-010: rule lowering not implemented) --------
    $tsContent = Get-Content $tsIr -Raw -ErrorAction SilentlyContinue
    if ($null -eq $tsContent) { $tsContent = "" }
    try {
        $tsJson = $tsContent | ConvertFrom-Json
        $tsNodeCount = 0
        if ($tsJson.nodes) { $tsNodeCount = @($tsJson.nodes).Count }
        if ($tsNodeCount -eq 0) {
            Write-Host "[SKIPPED] $name - TS reference emits empty IR (F-010: rule lowering not implemented)"
            $skipCount++
            continue
        }
    } catch {
        # TS IR is not valid JSON — fall through to comparison
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
    } elseif ($KnownDiffs -contains $name) {
        Write-Host "[KNOWN_DIFF] $name (F-007~F-012, deferred to v0.3.0)"
        $knownDiffCount++
    } else {
        Write-Host "[DIFF] $name (UNEXPECTED)"
        $unexpectedDiffCount++
    }
}

Write-Host ""
Write-Host "Diff-IR complete: $matchCount MATCH, $knownDiffCount KNOWN_DIFF, $unexpectedDiffCount unexpected DIFF, $skipCount SKIPPED"
exit $(if ($unexpectedDiffCount -gt 0) { 1 } else { 0 })
