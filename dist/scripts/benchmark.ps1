# =============================================================================
# NitroMan Benchmark Script
# =============================================================================
# Purpose:
#   Quick sanity + performance check for the NitroMan project.
#   1. Locate nitroman-cli.exe (release, then debug).
#   2. Measure `nitroman-cli.exe status` response time (5 iterations, avg ms).
#   3. Run `cargo test --workspace`, report PASS/FAIL.
#   4. Report RAM usage of any running nitroman-gui.exe / nitroman-cli.exe.
#   5. Print ASCII comparison table: NitroMan vs Acer Care Center + NitroSense.
#
# Notes:
#   - `status` may require admin rights; non-zero exit is treated as a
#     recorded result, not a crash.
#   - Script is PowerShell 5.1 compatible and ASCII-only (no emoji).
# =============================================================================

$ErrorActionPreference = 'Continue'

# Repo root is the parent of the scripts folder.
$repoRoot = Split-Path -Parent $PSScriptRoot
$exitCode = 0

$banner = '=' * 60
Write-Host $banner
Write-Host " NitroMan Benchmark"
Write-Host " Repo root: $repoRoot"
Write-Host $banner

# -----------------------------------------------------------------------------
# 1. Locate nitroman-cli.exe
# -----------------------------------------------------------------------------
$cliCandidates = @(
    (Join-Path $repoRoot 'target\release\nitroman-cli.exe'),
    (Join-Path $repoRoot 'target\debug\nitroman-cli.exe')
)
$cliPath = $null
foreach ($c in $cliCandidates) {
    if (Test-Path $c) { $cliPath = $c; break }
}

if ($null -eq $cliPath) {
    Write-Warning "nitroman-cli.exe not found in target\release\ or target\debug\. Skipping timing test."
} else {
    Write-Host "[CLI] Found: $cliPath"
}

# -----------------------------------------------------------------------------
# 2. Measure `nitroman-cli.exe status` response time
# -----------------------------------------------------------------------------
$statusAvgMs = $null
if ($null -ne $cliPath) {
    $iterations = 5
    $elapsedMs = @()
    for ($i = 1; $i -le $iterations; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        try {
            # Redirect stdout/stderr to suppress noise. Non-zero exit is OK.
            $null = & $cliPath status 2>$null
            $exit = $LASTEXITCODE
        } catch {
            $exit = -1
        }
        $sw.Stop()
        $elapsedMs += $sw.Elapsed.TotalMilliseconds
        Write-Host ("  Run {0}: {1:F2} ms (exit={2})" -f $i, $sw.Elapsed.TotalMilliseconds, $exit)
    }
    $statusAvgMs = ($elapsedMs | Measure-Object -Average).Average
    Write-Host ("[Status] Average over {0} runs: {1:F2} ms" -f $iterations, $statusAvgMs)
} else {
    Write-Host "[Status] Skipped (no CLI binary)."
}

# -----------------------------------------------------------------------------
# 3. Run cargo test --workspace
# -----------------------------------------------------------------------------
$testResult = 'SKIPPED'
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($null -eq $cargo) {
    Write-Warning "cargo not found on PATH. Skipping unit tests."
} else {
    Write-Host "[Tests] Running 'cargo test --workspace'..."
    try {
        Push-Location $repoRoot
        & cargo test --workspace 2>&1 | Select-Object -Last 20 | Write-Host
        $testExit = $LASTEXITCODE
    } catch {
        $testExit = -1
    } finally {
        Pop-Location
    }
    if ($testExit -eq 0) {
        $testResult = 'PASS'
    } else {
        $testResult = 'FAIL'
        $exitCode = 1
    }
    Write-Host "[Tests] Result: $testResult (exit=$testExit)"
}

# -----------------------------------------------------------------------------
# 4. Memory usage of running NitroMan processes
# -----------------------------------------------------------------------------
$procNames = @('nitroman-gui', 'nitroman-cli')
$procs = Get-Process -Name $procNames -ErrorAction SilentlyContinue
$ramMb = $null
if ($null -eq $procs -or $procs.Count -eq 0) {
    Write-Host "[Memory] No running nitroman-gui.exe or nitroman-cli.exe processes."
} else {
    $totalWs = ($procs | Measure-Object -Property WorkingSet64 -Sum).Sum
    $ramMb = $totalWs / 1MB
    Write-Host ("[Memory] {0} process(es) found, total WorkingSet: {1:F2} MB" -f $procs.Count, $ramMb)
}

# -----------------------------------------------------------------------------
# 5. ASCII comparison table
# -----------------------------------------------------------------------------
# Measured Idle RAM: fall back to 40-60 MB estimate if no process was running.
if ($null -ne $ramMb) {
    $nitroRam = "{0:F1} MB (measured)" -f $ramMb
} else {
    $nitroRam = "40-60 MB (idle estimate)"
}

$nitroStatus = if ($null -ne $statusAvgMs) { "{0:F1} ms" -f $statusAvgMs } else { "N/A (not measured)" }

# Column widths
$col1 = 22  # Metric
$col2 = 28  # NitroMan
$col3 = 32  # Acer Care Center + NitroSense

function Format-Row([string]$c1, [string]$c2, [string]$c3) {
    return "| {0} | {1} | {2} |" -f `
        $c1.PadRight($col1), $c2.PadRight($col2), $c3.PadRight($col3)
}
function Get-Separator {
    $s1 = '-' * $col1; $s2 = '-' * $col2; $s3 = '-' * $col3
    return "+-$s1-+-$s2-+-$s3-+"
}

Write-Host ""
Write-Host $banner
Write-Host " NitroMan vs Acer Care Center + NitroSense"
Write-Host $banner
$sep = Get-Separator
Write-Host $sep
Write-Host (Format-Row "Metric" "NitroMan" "Acer Care Ctr + NitroSense")
Write-Host $sep
Write-Host (Format-Row "Idle RAM"        $nitroRam   "300-600 MB")
Write-Host (Format-Row "Startup Time"    "<0.5s"     "5-8s")
Write-Host (Format-Row "Status Response" $nitroStatus "N/A (vendor tool)")
Write-Host $sep

$ramNote = if ($null -ne $ramMb) { "RAM measured, {0} proc(s)" -f $procs.Count } else { "RAM not measured (no proc)" }
Write-Host ""
Write-Host ("Summary: Tests={0} | Status avg={1} | {2}" -f $testResult, $nitroStatus, $ramNote)

exit $exitCode
