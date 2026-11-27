# usage: sample_ui.ps1 -Repo <repo> -Out <folded file> [-Samples 3000] [-Interval 5]
# Batch 4a gate 6 attribution: plays the demo song (CARTWAVE_AUTOPLAY) in the foreground and
# samples the UI thread's stacks with docs/perf/tools/Sampler.cs (no ETW, no elevation).
# Writes folded stacks ("a;b;c count") to -Out. Compare two builds with fold_diff.py.
# -DragY <client y>: sample during a drag instead (the time scrollbar's thumb: 890 on the
# baseline, 817 with the Main row at 1440 x 920).
param([string]$Repo, [string]$Out, [int]$Samples = 3000, [int]$Interval = 5, [int]$DragY = 0)
$Tools = Join-Path $PSScriptRoot "..\..\perf\tools"
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CARTWAVE_DEV = "1"; $env:CARTWAVE_AUTOPLAY = "1"
$env:CARTWAVE_DATA_DIR = Join-Path $env:TEMP "chipdaw-sample-ui"
$exe = Join-Path $Repo "target\release\cartwave.exe"
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
$sym = Join-Path $Repo "target\release"
if ($DragY) {
  # The 200-clip set (F4) while it plays, then a drag held at (650, DragY), swaying 160 px
  # sideways. The sampler runs in its own process while this one drags.
  Key 0x73; Start-Sleep -Seconds 2
  $job = Start-Job -ScriptBlock { param($t, $ap, $at, $n, $iv, $sy) Add-Type -Path $t; [Sampler]::Run($ap, $at, $n, $iv, $sy) } -ArgumentList (Join-Path $Tools "Sampler.cs"), $p.Id, $tid, $Samples, $Interval, $sym
  Move-To 650 $DragY; Down
  $i = 0
  while ($job.State -eq "Running") { Move-To (650 + [int](160 * [Math]::Sin($i / 12.0))) $DragY; $i++; Start-Sleep -Milliseconds 8 }
  Up
  $folded = Receive-Job $job | Out-String
} else {
  $folded = [Sampler]::Run($p.Id, $tid, $Samples, $Interval, $sym)
}
Stop-Process -Id $p.Id -Force
Set-Content -Path $Out -Value $folded -Encoding utf8
"front $front; tid $tid; lines $(($folded -split "`n").Count)"
