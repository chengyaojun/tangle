#requires -Version 5.1
<#
.SYNOPSIS
    Audit matrix driver: runs every (CLI surface x target x mode x fixture) cell.
.DESCRIPTION
    Outputs:
      tests/audit/output/<timestamp>/matrix.csv   - one row per cell
      tests/audit/output/<timestamp>/cells/<id>.out - per-cell stdout+stderr
      tests/audit/output/<timestamp>/summary.md    - failing cells grouped
#>
[CmdletBinding()]
param(
    [string]$OutputDir = (Join-Path $PSScriptRoot "output")
)

$ErrorActionPreference = "Continue"
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $OutputDir $timestamp
$cellsDir = Join-Path $runDir "cells"
New-Item -ItemType Directory -Force -Path $cellsDir | Out-Null

$csvPath = Join-Path $runDir "matrix.csv"
$summaryPath = Join-Path $runDir "summary.md"

# Fixtures
$examples = Get-ChildItem "examples\*.tangle.md" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
$tests = Get-ChildItem "tests\basic\*.tangle.md","tests\errors\*.tangle.md","tests\mvp\*.tangle.md","tests\rules\*.tangle.md","tests\structs\*.tangle.md" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
$fixtures = @($examples) + @($tests) | Where-Object { $_ -ne $null }

# Surfaces, targets, modes
$surfaces = @("run","build","doc")
$targets = @("js","py","go")
$modes = @("normal","incremental","interp")

"surface,target,mode,fixture,exit_code,diag_count,diag_codes" | Out-File -FilePath $csvPath -Encoding UTF8

$failingCells = [System.Collections.ArrayList]::new()
$cellCount = 0

foreach ($fixture in $fixtures) {
    foreach ($surface in $surfaces) {
        foreach ($target in $targets) {
            foreach ($mode in $modes) {
                # Skip invalid combinations
                if ($surface -eq "doc") {
                    if ($target -ne "js" -or $mode -ne "normal") { continue }
                }
                if ($surface -eq "build" -and $mode -eq "interp") { continue }
                if ($target -eq "py" -and $mode -eq "interp") { continue }
                if ($target -eq "go" -and $mode -eq "interp") { continue }
                # FIX: `run --interp` is not yet implemented in the CLI (see
                # compiler/tangle-cli/src/main.rs Run variant). Skip until the
                # CLI gains an --interp flag. Existing skips above already
                # exclude py+interp and go+interp; this completes interp skip.
                if ($surface -eq "run" -and $mode -eq "interp") { continue }

                $cellId = "${surface}_${target}_${mode}_$(Split-Path $fixture -Leaf)"
                $args = @($surface, $fixture)
                # FIX: `doc` subcommand does not accept --target (only --output).
                # Only run/build accept --target.
                if ($surface -ne "doc") { $args += @("--target", $target) }
                if ($mode -eq "incremental") { $args += "--incremental" }
                # NOTE: --interp branch below is currently dead code because of
                # the skip above; retained for when the CLI implements --interp.
                if ($surface -eq "run" -and $mode -eq "interp") { $args += "--interp" }

                $cellOutFile = Join-Path $cellsDir "$cellId.out"
                $stderrFile = Join-Path $cellsDir "$cellId.err"

                $process = Start-Process -FilePath "cargo" -ArgumentList (@("run","--quiet","--") + $args) -NoNewWindow -PassThru -Wait -RedirectStandardOutput $cellOutFile -RedirectStandardError $stderrFile
                $exitCode = $process.ExitCode

                $stderrContent = Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue
                $diagMatches = [regex]::Matches($stderrContent, 'error\[(TANGLE_[A-Z_]+)\]')
                $diagCount = $diagMatches.Count
                $diagCodes = ($diagMatches | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique) -join '|'

                $csvRow = "$surface,$target,$mode,$fixture,$exitCode,$diagCount,$diagCodes"
                Add-Content -Path $csvPath -Value $csvRow -Encoding UTF8

                if ($diagCount -gt 0) {
                    $failingCells.Add([PSCustomObject]@{
                        Cell = $cellId; Surface = $surface; Target = $target; Mode = $mode; Fixture = $fixture; DiagCount = $diagCount; Codes = $diagCodes
                    }) | Out-Null
                }
                $cellCount++
            }
        }
    }
}

# Add emit-ir cells
foreach ($fixture in $fixtures) {
    $cellId = "build_emit-ir_normal_$(Split-Path $fixture -Leaf)"
    $cellOutFile = Join-Path $cellsDir "$cellId.out"
    $stderrFile = Join-Path $cellsDir "$cellId.err"
    $process = Start-Process -FilePath "cargo" -ArgumentList @("run","--quiet","--","build",$fixture,"--emit-ir") -NoNewWindow -PassThru -Wait -RedirectStandardOutput $cellOutFile -RedirectStandardError $stderrFile
    $stderrContent = Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue
    $diagMatches = [regex]::Matches($stderrContent, 'error\[(TANGLE_[A-Z_]+)\]')
    $diagCount = $diagMatches.Count
    $diagCodes = ($diagMatches | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique) -join '|'
    Add-Content -Path $csvPath -Value "build,emit-ir,normal,$fixture,$($process.ExitCode),$diagCount,$diagCodes" -Encoding UTF8
    if ($diagCount -gt 0) {
        $failingCells.Add([PSCustomObject]@{ Cell = $cellId; Surface = "build"; Target = "emit-ir"; Mode = "normal"; Fixture = $fixture; DiagCount = $diagCount; Codes = $diagCodes }) | Out-Null
    }
    $cellCount++
}

# Summary
$summary = @"
# Audit Run Summary

- Timestamp: $timestamp
- Total cells: $cellCount
- Failing cells (with diagnostics): $($failingCells.Count)

## Failing cells

| Cell | Surface | Target | Mode | Fixture | DiagCount | Codes |
|------|---------|--------|------|---------|-----------|-------|
"@
foreach ($f in $failingCells) {
    $summary += "`n| $($f.Cell) | $($f.Surface) | $($f.Target) | $($f.Mode) | $(Split-Path $f.Fixture -Leaf) | $($f.DiagCount) | $($f.Codes) |"
}
$summary | Out-File -FilePath $summaryPath -Encoding UTF8

Write-Host "Audit complete: $cellCount cells, $($failingCells.Count) failing"
Write-Host "Run dir: $runDir"
Write-Host "Summary: $summaryPath"
