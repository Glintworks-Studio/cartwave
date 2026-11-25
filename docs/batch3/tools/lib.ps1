# Shared helpers for the Batch 3 harness (dot-source it). The same launch and
# input pattern as docs/shell/tools/at_shell.ps1: synthetic input into the
# real app, native dialogs through UI Automation (dialog.ps1), screenshots of
# the window (shot.ps1), project files checked afterwards (check_b3.py).
# Window 1440x900 logical; -Scale sets Settings > Display > Interface scale
# (through settings.json in the throw-away data dir).
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
. "$Repo\docs\shell\tools\dialog.ps1"
. "$Repo\docs\shell\tools\pixels.ps1"
$ErrorActionPreference = "Continue"
. "$Repo\docs\perf\tools\input.ps1" 2>$null
$ErrorActionPreference = "Stop"

# Logical -> client px at the current interface scale.
$global:S = 1.0
function P($x, $y) { @([int][math]::Round($x * $global:S), [int][math]::Round($y * $global:S)) }

function Launch([string]$Project = "", [string]$Data, [string[]]$AppArgs = @(), [int]$Scale = 100) {
  Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Milliseconds 300
  New-Item -ItemType Directory -Force $Data | Out-Null
  if ($Scale -ne 100) { "{ ""ui_scale"": $Scale }" | Set-Content -Encoding ascii (Join-Path $Data "settings.json") }
  $global:S = $Scale / 100.0
  $env:CHIPDAW_DATA_DIR = $Data; $env:CHIPDAW_AUDIO = "0"; $env:CHIPDAW_DEV = "1"
  if ($Project) { $env:CHIPDAW_PROJECT = $Project } else { Remove-Item Env:CHIPDAW_PROJECT -ErrorAction SilentlyContinue }
  $err = Join-Path $Data "stderr.txt"
  if ($AppArgs.Count -gt 0) { $script:proc = Start-Process $global:Exe -ArgumentList $AppArgs -PassThru -RedirectStandardError $err }
  else { $script:proc = Start-Process $global:Exe -PassThru -RedirectStandardError $err }
  for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $script:proc.Id).MainWindowHandle -ne 0) { break } }
  Start-Sleep -Milliseconds 1500
  $global:AppH = (Get-Process -Id $script:proc.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null
}
function Alive { [bool](Get-Process -Id $script:proc.Id -ErrorAction SilentlyContinue) }
function Title { (Get-Process -Id $script:proc.Id).MainWindowTitle }
function Stop-App { if (Alive) { Stop-Process -Id $script:proc.Id -Force; Start-Sleep -Milliseconds 400 } }
function Ctrl($vk) { KeyDown 0x11; Key $vk; KeyUp 0x11; Start-Sleep -Milliseconds 300 }
function CtrlShift($vk) { KeyDown 0x11; KeyDown 0x10; Key $vk; KeyUp 0x10; KeyUp 0x11; Start-Sleep -Milliseconds 300 }
function CtrlShiftExt($vk) { KeyDown 0x11; KeyDown 0x10; KeyExt $vk; KeyUp 0x10; KeyUp 0x11; Start-Sleep -Milliseconds 300 }
function ShiftKey($vk) { KeyDown 0x10; KeyExt $vk; KeyUp 0x10; Start-Sleep -Milliseconds 250 }
function K($vk) { Key $vk; Start-Sleep -Milliseconds 250 }
function KE($vk) { KeyExt $vk; Start-Sleep -Milliseconds 250 }
# Mouse at logical (x, y).
function MoveL($x, $y) { $p = P $x $y; Move-To $p[0] $p[1] }
function ClickL($x, $y) { MoveL $x $y; Down; Up; Start-Sleep -Milliseconds 250 }
function ShiftClickL($x, $y) { MoveL $x $y; KeyDown 0x10; Down; Up; KeyUp 0x10; Start-Sleep -Milliseconds 250 }
function CtrlClickL($x, $y) { MoveL $x $y; KeyDown 0x11; Down; Up; KeyUp 0x11; Start-Sleep -Milliseconds 250 }
function DblL($x, $y) { MoveL $x $y; Down; Up; Start-Sleep -Milliseconds 60; Down; Up; Start-Sleep -Milliseconds 400 }
function RightL($x, $y) { MoveL $x $y; [In]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40; [In]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 400 }
function DragL($x0, $y0, $x1, $y1) { $a = P $x0 $y0; $b = P $x1 $y1; Move-To $a[0] $a[1]; Down; for ($i = 1; $i -le 16; $i++) { Move-To ([int]($a[0] + ($b[0] - $a[0]) * $i / 16)) ([int]($a[1] + ($b[1] - $a[1]) * $i / 16)); Start-Sleep -Milliseconds 15 }; Up; Start-Sleep -Milliseconds 300 }
function TypeText($t) { foreach ($ch in $t.ToCharArray()) { $vk = [int][char]::ToUpper($ch); if ($ch -eq ' ') { $vk = 0x20 }; if ([char]::IsUpper($ch)) { KeyDown 0x10; Key $vk; KeyUp 0x10 } else { Key $vk } }; Start-Sleep -Milliseconds 200 }
function Shot($n) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$global:Work\$n.png" | Out-Null }
# Ctrl+S into the project the app was opened with (CHIPDAW_PROJECT keeps its path).
function Save { Ctrl 0x53; Start-Sleep -Milliseconds 600 }
function StdErr($d) { Get-Content (Join-Path $d "stderr.txt") -ErrorAction SilentlyContinue }
