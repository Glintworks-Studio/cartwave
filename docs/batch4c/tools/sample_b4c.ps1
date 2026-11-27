# usage: sample_b4c.ps1 -Exe <cartwave.exe|app.exe> -Out <folded file> [-Project <file>] [-Samples 4000] [-Interval 4] [-Thread ui|audio] [-Env K=V;...]
# Batch 4c §2: batch 4a's sample_ui.ps1 for any build and project: plays -Project (autoplay) at
# 1440 x 920 and samples one thread's stacks with docs/perf/tools/Sampler.cs (no ETW, no
# elevation). -Thread ui (the process's first thread) or audio (the busiest other thread, the
# cpal callback's). Sets both CARTWAVE_* and CHIPDAW_* names (pre-rename builds). Folded stacks
# ("a;b;c count") to -Out; compare with docs/batch4a/tools/fold_diff.py.
param([string]$Exe, [string]$Out, [string]$Project = "", [int]$Samples = 4000, [int]$Interval = 4, [string]$Thread = "ui", [string]$Env = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Tools = Join-Path $Repo "docs\perf\tools"
if (-not $Project) { $Project = Join-Path $Repo "docs\batch4a\songs\stress64.chipdaw" }
$Exe = (Resolve-Path $Exe).Path
Get-Process -Name cartwave, app -ErrorAction SilentlyContinue | Stop-Process -Force
function Set-Both($k, $v) { Set-Item "Env:CARTWAVE_$k" $v; Set-Item "Env:CHIPDAW_$k" $v }
Set-Both DEV 1; Set-Both AUTOPLAY 1; Set-Both PROJECT $Project; Set-Both DATA_DIR (Join-Path $env:TEMP "cartwave-b4c-sample")
if ($Env) { foreach ($kv in $Env.Split(";")) { $k, $v = $kv.Split("=", 2); Set-Both $k $v } }
$p = Start-Process $Exe -ArgumentList "--size", "1440x920" -PassThru -WorkingDirectory (Split-Path (Split-Path (Split-Path $Exe)))
Start-Sleep -Seconds 5
$p.Refresh()
if ($Thread -eq "ui") {
  $tid = ($p.Threads | Sort-Object StartTime | Select-Object -First 1).Id
} else {
  $c0 = @{}; foreach ($th in $p.Threads) { try { $c0[$th.Id] = $th.TotalProcessorTime.TotalMilliseconds } catch {} }
  Start-Sleep -Seconds 2; $p.Refresh()
  $ui = ($p.Threads | Sort-Object StartTime | Select-Object -First 1).Id
  $tid = ($p.Threads | Where-Object { $_.Id -ne $ui -and $c0.ContainsKey($_.Id) } | Sort-Object { $_.TotalProcessorTime.TotalMilliseconds - $c0[$_.Id] } -Descending | Select-Object -First 1).Id
}
Add-Type -Path (Join-Path $Tools "Sampler.cs") -ErrorAction SilentlyContinue
$folded = [Sampler]::Run($p.Id, $tid, $Samples, $Interval, (Split-Path $Exe))
Stop-Process -Id $p.Id -Force
Set-Content -Path $Out -Value $folded -Encoding utf8
"tid $tid; lines $(($folded -split "`n").Count)"
