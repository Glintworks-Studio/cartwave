# usage: idle_b4c.ps1 -Builds name=exe,... [-Rounds 3] [-Secs 20]
# Batch 4c §2 (the idle row): each build opens the demo (as bench_b4a's idle step: no project, not
# playing, the mouse parked over the window), waits 5 s, then for -Secs counts presented frames
# (the frame log), process cycles (QueryProcessCycleTime: precise, unlike the tick-sampled CPU
# times, which move in 15.6 ms quanta: 0.08 % of 20 s each) and process CPU.
param([string]$Builds, [int]$Rounds = 3, [int]$Secs = 20, [string]$Work = (Join-Path $env:TEMP "cartwave-b4c-idle"))
New-Item -ItemType Directory -Force $Work | Out-Null
Add-Type -Namespace B4C -Name Pc -MemberDefinition '[DllImport("kernel32.dll")] public static extern bool QueryProcessCycleTime(IntPtr h, out ulong c);' -ErrorAction SilentlyContinue
$list = $Builds.Split(",") | ForEach-Object { $n, $e = $_.Split("=", 2); [pscustomobject]@{ Name = $n; Exe = (Resolve-Path $e).Path } }
function Set-Both($k, $v) { Set-Item "Env:CARTWAVE_$k" $v; Set-Item "Env:CHIPDAW_$k" $v }
for ($r = 1; $r -le $Rounds; $r++) {
  $order = @($list); if (-not ($r % 2)) { [array]::Reverse($order) }
  foreach ($b in $order) {
    Get-Process -Name cartwave, app -ErrorAction SilentlyContinue | Stop-Process -Force
    $data = Join-Path $Work "data-$($b.Name)"; Remove-Item -Recurse -Force $data -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force $data | Out-Null
    $fl = Join-Path $Work "$($b.Name)-$r.frames"
    Set-Both DEV 1; Set-Both DATA_DIR $data; Set-Both FRAMELOG $fl
    foreach ($k in "PROJECT", "AUTOPLAY") { Remove-Item "Env:CARTWAVE_$k", "Env:CHIPDAW_$k" -ErrorAction SilentlyContinue }
    $p = Start-Process $b.Exe -ArgumentList "--size", "1440x920" -PassThru -WorkingDirectory (Split-Path (Split-Path (Split-Path $b.Exe)))
    Start-Sleep -Seconds 3
    Start-Sleep -Seconds 2
    $n0 = (Get-Content $fl -ErrorAction SilentlyContinue | Measure-Object -Line).Lines
    $p.Refresh(); $c0 = $p.TotalProcessorTime; $sw = [Diagnostics.Stopwatch]::StartNew(); $y0 = [uint64]0; [B4C.Pc]::QueryProcessCycleTime($p.Handle, [ref]$y0) | Out-Null
    Start-Sleep -Seconds $Secs
    $y1 = [uint64]0; [B4C.Pc]::QueryProcessCycleTime($p.Handle, [ref]$y1) | Out-Null
    $p.Refresh(); $cpu = [math]::Round(($p.TotalProcessorTime - $c0).TotalMilliseconds / $sw.Elapsed.TotalMilliseconds * 100, 2)
    $mcs = [math]::Round(($y1 - $y0) / $sw.Elapsed.TotalSeconds / 1e6, 2)
    Stop-Process -Id $p.Id -Force
    Start-Sleep -Milliseconds 300
    $n1 = (Get-Content $fl -ErrorAction SilentlyContinue | Measure-Object -Line).Lines
    "r$r $($b.Name): $mcs M cycles/s, cpu $cpu %, frames during the window ~$($n1 - $n0) (the log flushes every 30 frames)"
    foreach ($k in "DEV", "DATA_DIR", "FRAMELOG") { Remove-Item "Env:CARTWAVE_$k", "Env:CHIPDAW_$k" -ErrorAction SilentlyContinue }
  }
}
