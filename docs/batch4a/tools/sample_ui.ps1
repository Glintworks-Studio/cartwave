# usage: sample_ui.ps1 -Repo <repo> -Out <folded file> [-Samples 3000] [-Interval 5]
# Batch 4a gate 6 attribution: plays the demo song (CHIPDAW_AUTOPLAY) in the foreground and
# samples the UI thread's stacks with docs/perf/tools/Sampler.cs (no ETW, no elevation).
# Writes folded stacks ("a;b;c count") to -Out. Compare two builds with fold_diff.py.
param([string]$Repo, [string]$Out, [int]$Samples = 3000, [int]$Interval = 5)
$Tools = Join-Path $PSScriptRoot "..\..\perf\tools"
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CHIPDAW_DEV = "1"; $env:CHIPDAW_AUTOPLAY = "1"
$env:CHIPDAW_DATA_DIR = Join-Path $env:TEMP "chipdaw-sample-ui"
$exe = Join-Path $Repo "target\release\app.exe"
$p = Start-Process $exe -ArgumentList "--size", "1440x920" -PassThru -WorkingDirectory $Repo
Start-Sleep -Seconds 4
$p.Refresh(); $global:AppH = $p.MainWindowHandle
. (Join-Path $Tools "input.ps1")
Add-Type -Namespace SampFg -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);' -ErrorAction SilentlyContinue
for ($t = 0; $t -lt 5 -and [SampFg.W]::GetForegroundWindow() -ne $global:AppH; $t++) {
  [In]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [In]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
  [SampFg.W]::SetForegroundWindow($global:AppH) | Out-Null; Start-Sleep -Milliseconds 200
}
$front = [SampFg.W]::GetForegroundWindow() -eq $global:AppH
Move-To 1300 700
# The UI thread is the process's first thread.
$tid = ($p.Threads | Sort-Object StartTime | Select-Object -First 1).Id
Add-Type -Path (Join-Path $Tools "Sampler.cs") -ErrorAction SilentlyContinue
$folded = [Sampler]::Run($p.Id, $tid, $Samples, $Interval, (Join-Path $Repo "target\release"))
Stop-Process -Id $p.Id -Force
Set-Content -Path $Out -Value $folded -Encoding utf8
"front $front; tid $tid; lines $(($folded -split "`n").Count)"
