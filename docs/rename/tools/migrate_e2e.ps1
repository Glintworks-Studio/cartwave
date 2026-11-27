# usage: migrate_e2e.ps1 [-Exe .\target\release\cartwave.exe] [-Out <dir>]
# The rename's settings migration (RENAME_PROMPT.md §1.4, §3.3) on the real app: a fake
# %APPDATA% (APPDATA points at a temp folder, so the user's real folders are never touched)
# holds a pre-rename ChipDAW folder: settings (a non-default autosave, a recent file), user
# presets, a recovery file and a session marker left by a crash. Launch 1 must copy it to
# Cartwave (the recovery file renamed), keep the old folder, and offer the recovery; launch 2
# must remove the old folder and the marker, and keep the settings.
param([string]$Exe = ".\target\release\cartwave.exe", [string]$Out = (Join-Path $env:TEMP "rn-migrate"))
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$Exe = (Resolve-Path $Exe).Path
Remove-Item -Recurse -Force $Out -ErrorAction SilentlyContinue
$fake = Join-Path $Out "AppData"
$old = Join-Path $fake "ChipDAW"; $new = Join-Path $fake "Cartwave"
New-Item -ItemType Directory -Force (Join-Path $old "sub") | Out-Null
'{ "autosave_secs": 300, "recent": ["C:/songs/old.chipdaw"], "show_overview": false }' | Set-Content -Encoding ascii (Join-Path $old "settings.json")
'[]' | Set-Content -Encoding ascii (Join-Path $old "user_presets.json")
Copy-Item (Join-Path $Repo "docs\songs\v7\demo_band.chipdaw") (Join-Path $old "recovery.chipdaw")
'1234' | Set-Content -Encoding ascii (Join-Path $old "session.lock")
'x' | Set-Content -Encoding ascii (Join-Path $old "sub\keep.txt")
foreach ($v in Get-ChildItem Env: | Where-Object { $_.Name -like "CARTWAVE_*" -or $_.Name -like "CHIPDAW_*" }) { Remove-Item "Env:$($v.Name)" }
$savedAppData = $env:APPDATA
$env:APPDATA = $fake; $env:CARTWAVE_AUDIO = "0"; $env:CARTWAVE_DEV = "1"
$obs = [ordered]@{}
function Launch($n) {
  $p = Start-Process $Exe -PassThru -RedirectStandardError (Join-Path $Out "launch$n.stderr.txt")
  for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $p.Id).MainWindowHandle -ne 0) { break } }
  Start-Sleep -Milliseconds 2500
  & pwsh -NoProfile -File (Join-Path $Repo "docs\perf\tools\shot.ps1") -Out (Join-Path $Out "launch$n.png") | Out-Null
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 500
}
Launch 1
$obs.l1_new_exists = Test-Path $new
$obs.l1_old_kept = Test-Path (Join-Path $old "settings.json")
$obs.l1_marker = if (Test-Path (Join-Path $new "migrated-from-chipdaw.txt")) { (Get-Content (Join-Path $new "migrated-from-chipdaw.txt"))[1] } else { $null }
$obs.l1_recovery_renamed = (Test-Path (Join-Path $new "recovery.cartwave")) -and -not (Test-Path (Join-Path $new "recovery.chipdaw"))
$obs.l1_sub_copied = Test-Path (Join-Path $new "sub\keep.txt")
$obs.l1_no_staging = -not (Test-Path (Join-Path $fake "Cartwave.moving"))
Launch 2
$obs.l2_old_removed = -not (Test-Path $old)
$obs.l2_marker_removed = -not (Test-Path (Join-Path $new "migrated-from-chipdaw.txt"))
$st = Get-Content -Raw (Join-Path $new "settings.json") | ConvertFrom-Json
$obs.l2_settings_kept = ($st.autosave_secs -eq 300) -and ($st.recent -contains "C:/songs/old.chipdaw")
$obs.l2_stderr_removed_line = [bool](Select-String -Path (Join-Path $Out "launch2.stderr.txt") -Pattern "Removed the old app-data folder" -Quiet)
Launch 3
$obs.l3_nothing_moves_again = -not (Test-Path $old)
$env:APPDATA = $savedAppData
Remove-Item Env:CARTWAVE_AUDIO, Env:CARTWAVE_DEV -ErrorAction SilentlyContinue
$obs | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $Out "obs.json")
$ok = $obs.l1_new_exists -and $obs.l1_old_kept -and $obs.l1_marker -eq "launched" -and $obs.l1_recovery_renamed -and $obs.l1_sub_copied -and $obs.l1_no_staging -and $obs.l2_old_removed -and $obs.l2_marker_removed -and $obs.l2_settings_kept -and $obs.l2_stderr_removed_line -and $obs.l3_nothing_moves_again
$obs.GetEnumerator() | ForEach-Object { "{0} = {1}" -f $_.Key, $_.Value }
if ($ok) { "PASS: the app-data move (launch 1 copies and keeps the old folder; launch 2 removes it)" } else { "FAIL: see obs.json" }
