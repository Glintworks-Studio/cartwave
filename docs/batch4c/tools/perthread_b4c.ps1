# usage: perthread_b4c.ps1 -Builds name=exe,... [-Project <file>] [-Rounds 3] [-Secs 20]
# Batch 4c §2: CPU per thread (ms of CPU per s, i.e. % of a core) while -Project plays at
# 1440 x 920, for each build, interleaved. Threads are named by their start address's module
# (Win32 QueryInformationThread / ThreadQuerySetWin32StartAddress, documented in winternl.h, via
# NtQueryInformationThread) so the UI thread, the audio thread and the driver's threads can be
# told apart across builds. Prints one table per run, threads above 0.2 %.
param([string]$Builds, [string]$Project = "", [int]$Rounds = 3, [int]$Secs = 20, [string]$Out = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
if (-not $Project) { $Project = Join-Path $Repo "docs\batch4a\songs\stress64.chipdaw" }
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text; using System.Diagnostics;
public static class TS {
  [DllImport("kernel32.dll")] public static extern IntPtr OpenThread(int a, bool i, int id);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
  [DllImport("ntdll.dll")] public static extern int NtQueryInformationThread(IntPtr h, int c, out IntPtr v, int l, IntPtr r);
  public static long Start(int tid) {
    IntPtr h = OpenThread(0x0040, false, tid); if (h == IntPtr.Zero) return 0;
    IntPtr a; NtQueryInformationThread(h, 9, out a, IntPtr.Size, IntPtr.Zero); CloseHandle(h); return a.ToInt64();
  }
}
"@
function Module-Of($p, [long]$addr) {
  foreach ($m in $p.Modules) { $b = $m.BaseAddress.ToInt64(); if ($addr -ge $b -and $addr -lt $b + $m.ModuleMemorySize) { return "$($m.ModuleName)+0x{0:X}" -f ($addr - $b) } }
  "?"
}
function Set-Both($k, $v) { Set-Item "Env:CARTWAVE_$k" $v; Set-Item "Env:CHIPDAW_$k" $v }
$list = $Builds.Split(",") | ForEach-Object { $n, $e = $_.Split("=", 2); [pscustomobject]@{ Name = $n; Exe = (Resolve-Path $e).Path } }
$all = @()
for ($r = 1; $r -le $Rounds; $r++) {
  $order = @($list); if (-not ($r % 2)) { [array]::Reverse($order) }
  foreach ($b in $order) {
    Get-Process -Name cartwave, app -ErrorAction SilentlyContinue | Stop-Process -Force
    $data = Join-Path $env:TEMP "cartwave-b4c-pt-$($b.Name)"; Remove-Item -Recurse -Force $data -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force $data | Out-Null
    Set-Both DEV 1; Set-Both DATA_DIR $data; Set-Both AUTOPLAY 1; Set-Both PROJECT $Project
    $p = Start-Process $b.Exe -ArgumentList "--size", "1440x920" -PassThru -WorkingDirectory (Split-Path (Split-Path (Split-Path $b.Exe)))
    Start-Sleep -Seconds 5
    $p.Refresh()
    $t0 = @{}; $name = @{}; $k = 0
    foreach ($th in ($p.Threads | Sort-Object StartTime)) {
      try { $t0[$th.Id] = $th.TotalProcessorTime.TotalMilliseconds; $name[$th.Id] = "#$k " + (Module-Of $p ([TS]::Start($th.Id))) } catch {}
      $k++
    }
    $sw = [Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Seconds $Secs; $p.Refresh()
    $el = $sw.Elapsed.TotalMilliseconds
    foreach ($th in $p.Threads) {
      if ($t0.ContainsKey($th.Id)) {
        try { $pct = ($th.TotalProcessorTime.TotalMilliseconds - $t0[$th.Id]) / $el * 100 } catch { continue }
        $all += [pscustomobject]@{ Round = $r; Build = $b.Name; Thread = $name[$th.Id]; Pct = [math]::Round($pct, 2) }
      }
    }
    Stop-Process -Id $p.Id -Force
    foreach ($kk in "DEV", "DATA_DIR", "AUTOPLAY", "PROJECT") { Remove-Item "Env:CARTWAVE_$kk", "Env:CHIPDAW_$kk" -ErrorAction SilentlyContinue }
  }
}
# Mean per (build, thread module without the instance number), summed over same-named threads.
$agg = $all | ForEach-Object { $_ | Add-Member -PassThru Key (($_.Thread -replace '^#\d+ ', '')) } |
  Group-Object Build, Round, Key | ForEach-Object { [pscustomobject]@{ Build = $_.Group[0].Build; Round = $_.Group[0].Round; Key = $_.Group[0].Key; Pct = ($_.Group | Measure-Object Pct -Sum).Sum } } |
  Group-Object Build, Key | ForEach-Object { [pscustomobject]@{ Build = $_.Group[0].Build; Thread = $_.Group[0].Key; MeanPct = [math]::Round((($_.Group | Measure-Object Pct -Sum).Sum / $Rounds), 2) } } |
  Where-Object { $_.MeanPct -ge 0.1 } | Sort-Object Thread, Build
$agg | Format-Table -AutoSize | Out-String -Width 200
if ($Out) { $all | Export-Csv -NoTypeInformation $Out }
