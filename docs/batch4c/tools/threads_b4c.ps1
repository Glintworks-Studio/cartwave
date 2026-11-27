# usage: threads_b4c.ps1 -Builds name=exe,... [-Project <file>] [-Rounds 3] [-Secs 30] [-Work <dir>] [-Env K=V;K=V]
# Batch 4c §2 (locating the 4b excess): per-thread cost, with less noise than process CPU. Each
# build plays -Project (autoplay) at 1440 x 920 for -Secs with CARTWAVE_AUDIOLOG on; the log's
# per-thread TSC cycle counts (QueryThreadCycleTime) give UI kcycles per presented frame and
# audio kcycles per callback; process CPU (% of a core) is measured beside them. Builds run
# interleaved, the order alternating per round. Sets both CARTWAVE_* and CHIPDAW_* names, so
# pre-rename builds (app.exe) run the same scenario. Appends one line per run to <Work>\threads.tsv.
param([string]$Builds, [string]$Project = "", [int]$Rounds = 3, [int]$Secs = 30, [string]$Work = (Join-Path $env:TEMP "cartwave-b4c-threads"), [string]$Env = "", [string]$Tag = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
if (-not $Project) { $Project = Join-Path $Repo "docs\batch4a\songs\stress64.chipdaw" }
New-Item -ItemType Directory -Force $Work | Out-Null
$list = $Builds.Split(",") | ForEach-Object { $n, $e = $_.Split("=", 2); [pscustomobject]@{ Name = $n; Exe = (Resolve-Path $e).Path } }
$tsv = Join-Path $Work "threads.tsv"
if (-not (Test-Path $tsv)) { "tag`tround`tbuild`tsecs`tui_kcyc_per_frame`taudio_kcyc_per_cb`tframes`tproc_cpu_pct`tstarved" | Set-Content $tsv }
function Set-Both($k, $v) { Set-Item "Env:CARTWAVE_$k" $v; Set-Item "Env:CHIPDAW_$k" $v }
function Clear-Both($k) { Remove-Item "Env:CARTWAVE_$k", "Env:CHIPDAW_$k" -ErrorAction SilentlyContinue }
for ($r = 1; $r -le $Rounds; $r++) {
  $order = @($list); if (-not ($r % 2)) { [array]::Reverse($order) }
  foreach ($b in $order) {
    Get-Process -Name cartwave, app -ErrorAction SilentlyContinue | Stop-Process -Force
    $log = Join-Path $Work "$Tag$($b.Name)-$r.csv"
    $data = Join-Path $Work "data-$($b.Name)"; Remove-Item -Recurse -Force $data -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force $data | Out-Null
    Set-Both DEV 1; Set-Both AUDIOLOG $log; Set-Both FRAMELOG ($log -replace "\.csv$", ".frames"); Set-Both QUIT_AFTER ($Secs + 4); Set-Both DATA_DIR $data; Set-Both AUTOPLAY 1; Set-Both PROJECT $Project
    if ($Env) { foreach ($kv in $Env.Split(";")) { $k, $v = $kv.Split("=", 2); Set-Both $k $v } }
    $p = Start-Process $b.Exe -ArgumentList "--size", "1440x920" -PassThru -WorkingDirectory (Split-Path (Split-Path (Split-Path $b.Exe)))
    Start-Sleep -Seconds 4
    $p.Refresh(); $c0 = $p.TotalProcessorTime; $sw = [Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds ($Secs - 2)
    $p.Refresh(); $cpu = [math]::Round(($p.TotalProcessorTime - $c0).TotalMilliseconds / $sw.Elapsed.TotalMilliseconds * 100, 2)
    $p.WaitForExit(30000) | Out-Null
    foreach ($k in "DEV", "FRAMELOG", "AUDIOLOG", "QUIT_AFTER", "DATA_DIR", "AUTOPLAY", "PROJECT") { Clear-Both $k }
    if ($Env) { foreach ($kv in $Env.Split(";")) { Clear-Both ($kv.Split("=", 2)[0]) } }
    # Per-second lines: skip the first 4 s (loading), average the rest.
    $rows = Import-Csv $log | Where-Object { $_.t_s -and $_.t_s -notlike "#*" -and [double]$_.t_s -ge 5 }
    $ui = ($rows | Measure-Object -Property ui_kcycles_per_frame -Average).Average
    $au = ($rows | Measure-Object -Property audio_kcycles_per_cb -Average).Average
    $st = ($rows | Select-Object -Last 1).starved
    "$Tag`t$r`t$($b.Name)`t$($rows.Count)`t$([math]::Round($ui,1))`t$([math]::Round($au,1))`t`t$cpu`t$st" | Add-Content $tsv
    "$Tag r$r $($b.Name): ui $([math]::Round($ui,1)) k/frame, audio $([math]::Round($au,1)) k/cb, cpu $cpu %"
  }
}
