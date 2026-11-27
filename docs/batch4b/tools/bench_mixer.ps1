# usage: bench_mixer.ps1 -Label <run> -Repo <repo> [-Secs 12] [-Work <dir>]
# Batch 4b §9.6, the new mixer scenarios (this branch only: the baseline has no mixer). The
# 64-studio-track project (batch 4a's stress64) playing from launch; Tab shows the mixer (64
# strips, the Main strip); then: the mixer while it plays, and the strips scrolled sideways (the
# wheel, a notch per step, right then left) while it plays. For a like-for-like line, the same
# project's arrangement view while it plays is measured first in the same run.
# Process CPU in % of one core; one line per run in <Work>\bench_mixer_log.txt, with proof the
# mixer was shown (the dev log's "view: mixer") and that 64 tracks were loaded.
param([string]$Label = "run", [int]$Secs = 12, [string]$Repo,
  [string]$Work = (Join-Path $env:TEMP "chipdaw-b4b-perf"))
New-Item -ItemType Directory -Force $Work | Out-Null
$d = Join-Path $Work "data-mixer-$Label"
Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $d | Out-Null
foreach ($k in "CHIPDAW_FROZEN_METERS", "CHIPDAW_AUDIO") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$env:CHIPDAW_DATA_DIR = $d; $env:CHIPDAW_DEV = "1"
$env:CHIPDAW_PROJECT = (Join-Path $Repo "docs\batch4a\songs\stress64.chipdaw"); $env:CHIPDAW_AUTOPLAY = "1"
Set-Location $Repo
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$err = Join-Path $Work "bench_mixer_err.txt"
$p = Start-Process ".\target\release\app.exe" -ArgumentList "--size", "1440x920" -PassThru -RedirectStandardError $err
Start-Sleep -Seconds 4
Remove-Item Env:CHIPDAW_PROJECT, Env:CHIPDAW_AUTOPLAY
. "$Repo\docs\perf\tools\input.ps1"
Add-Type -Namespace B4B -Name Fg -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);' -ErrorAction SilentlyContinue
$p.Refresh(); $global:AppH = $p.MainWindowHandle
function App-Front { [B4B.Fg]::GetForegroundWindow() -eq $global:AppH }
function Focus-App {
  for ($t = 0; $t -lt 5 -and -not (App-Front); $t++) {
    [In]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [In]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    [B4B.Fg]::ShowWindow($global:AppH, 5) | Out-Null; [B4B.Fg]::SetForegroundWindow($global:AppH) | Out-Null
    Start-Sleep -Milliseconds 200
  }
  App-Front
}
$lost = 0
$Res = [ordered]@{}
function Measure-Cpu([scriptblock]$during) {
  $p.Refresh(); $c0 = $p.TotalProcessorTime; $t0 = [Diagnostics.Stopwatch]::StartNew(); $i = 0
  while ($t0.Elapsed.TotalSeconds -lt $Secs) { & $during $i; $i++; Start-Sleep -Milliseconds 8 }
  $p.Refresh(); [math]::Round(($p.TotalProcessorTime - $c0).TotalMilliseconds / $t0.Elapsed.TotalMilliseconds * 100, 1)
}
if (-not (Focus-App)) { $lost++ }
Move-To 1300 500
$Res["s64_arrangement"] = Measure-Cpu { param($i) }
if (App-Front) { Key 0x09 } else { $lost++ }
Start-Sleep -Seconds 2
Move-To 1300 500
$Res["s64_mixer"] = Measure-Cpu { param($i) }
Move-To 700 400
$Res["s64_mixer_scroll"] = Measure-Cpu { param($i) $n = if (($i % 60) -lt 30) { -120 } else { 120 }; [In]::mouse_event(0x0800, 0, 0, $n, [UIntPtr]::Zero) }
pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out (Join-Path $Work "mixer-$Label.png") | Out-Null
Stop-Process -Id $p.Id -Force
$e = Get-Content $err -Raw -ErrorAction SilentlyContinue
$proof = if ($e -match "stress64.chipdaw \(64 clips\)" -and $e -match "view mixer") { "64 tracks, mixer shown" } else { "MIXER SCENARIO NOT REACHED" }
$line = "{0,-16} " -f $Label + (($Res.Keys | ForEach-Object { "{0} {1,5}%" -f $_, $Res[$_] }) -join "  ") + "  $proof" + $(if ($lost) { "  FOCUS LOST x${lost}: INVALID" } else { "  focus held" })
Add-Content (Join-Path $Work "bench_mixer_log.txt") $line
$line
