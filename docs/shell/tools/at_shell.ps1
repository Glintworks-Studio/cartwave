# usage: at_shell.ps1 [-Exe .\target\release\cartwave.exe] [-Work <dir>] [-Only AT-01,AT-03]
# Acceptance tests for Batches 1 and 2 (UX.md §6): AT-01, 02, 03, 04, 06, 15, 16, 18, 19.
# Synthetic input as in docs/stabilize/tools/scenario_v11.ps1; the native dialogs are found
# and answered through UI Automation (dialog.ps1), the menus are driven with their Alt
# mnemonics. Every step records what it saw (window titles, dialog texts, UIA label texts)
# in obs.json, saves project files and screenshots into the work dir; check_shell.py then
# checks obs.json and the saved files and prints PASS/FAIL per test.
# Window 1440x900 at 100%, audio off, a throw-away data dir per test.
param([string]$Exe = ".\target\release\cartwave.exe",
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
  [string]$Work = (Join-Path $env:TEMP ("chipdaw-at-shell-" + (Get-Date -Format "MMdd-HHmmss"))),
  [string[]]$Only = @())
$ErrorActionPreference = "Stop"
Set-Location $Repo
$Exe = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Force $Work | Out-Null
$obs = [ordered]@{ work = $Work }
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
. "$Repo\docs\shell\tools\dialog.ps1"
. "$Repo\docs\shell\tools\pixels.ps1"
# The input helpers, at script scope (no app runs yet: the window handle is set per launch).
$ErrorActionPreference = "Continue"
. "$Repo\docs\perf\tools\input.ps1" 2>$null
$ErrorActionPreference = "Stop"

function Want($id) { $Only.Count -eq 0 -or $Only -contains $id }
function Fresh($name) { $p = Join-Path $Work "$name.chipdaw"; Copy-Item "$Repo\docs\songs\v2\demo_band.chipdaw" $p -Force; $p }
function Launch([string]$Project = "", [string]$Data, [switch]$NoDev, [string[]]$AppArgs = @()) {
  Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Milliseconds 300
  $env:CARTWAVE_DATA_DIR = $Data; $env:CARTWAVE_AUDIO = "0"
  if ($NoDev) { Remove-Item Env:CARTWAVE_DEV -ErrorAction SilentlyContinue } else { $env:CARTWAVE_DEV = "1" }
  if ($Project) { $env:CARTWAVE_PROJECT = $Project } else { Remove-Item Env:CARTWAVE_PROJECT -ErrorAction SilentlyContinue }
  if ($AppArgs.Count -gt 0) { $script:proc = Start-Process $Exe -ArgumentList $AppArgs -PassThru } else { $script:proc = Start-Process $Exe -PassThru }
  for ($i = 0; $i -lt 50; $i++) { Start-Sleep -Milliseconds 100; if ((Get-Process -Id $script:proc.Id).MainWindowHandle -ne 0) { break } }
  Start-Sleep -Milliseconds 1500
  $global:AppH = (Get-Process -Id $script:proc.Id).MainWindowHandle
  [In]::SetForegroundWindow($global:AppH) | Out-Null
}
function Alive { [bool](Get-Process -Id $script:proc.Id -ErrorAction SilentlyContinue) }
function Title { (Get-Process -Id $script:proc.Id).MainWindowTitle }
function Ctrl($vk) { KeyDown 0x11; Key $vk; KeyUp 0x11; Start-Sleep -Milliseconds 300 }
function CtrlShift($vk) { KeyDown 0x11; KeyDown 0x10; Key $vk; KeyUp 0x10; KeyUp 0x11; Start-Sleep -Milliseconds 300 }
function AltF4 { KeyDown 0x12; Key 0x73; KeyUp 0x12; Start-Sleep -Milliseconds 800 }
function Click($x, $y) { Move-To $x $y; Down; Up; Start-Sleep -Milliseconds 250 }
function DelKey { KeyExt 0x2E; Start-Sleep -Milliseconds 400 }
function Shot($n) { pwsh -NoProfile -File "$Repo\docs\perf\tools\shot.ps1" -Out "$Work\$n.png" | Out-Null }
# Label texts of the app's own UI, through vizia's accessibility tree.
function Ui-Texts {
  $c = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)
  (Main-Window).FindAll([System.Windows.Automation.TreeScope]::Descendants, $c) | ForEach-Object { $_.Current.Name }
  # Text fields: their value.
  $e = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit)
  (Main-Window).FindAll([System.Windows.Automation.TreeScope]::Descendants, $e) | ForEach-Object {
    $v = $null
    if ($_.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$v)) { "field:" + $v.Current.Value }
  }
}
function Stop-App { if (Alive) { Stop-Process -Id $script:proc.Id -Force; Start-Sleep -Milliseconds 400 } }

# Client coordinates (1440x920 at 100%, the Option D layout of the design pass): the clips "Intro Lead" and "Hook A" on Pulse 1,
# empty Pulse 2 space, the browser search box, the inspector's CLIP field.
$IntroLead = @(700, 140); $HookA = @(1100, 140); $EmptyP2 = @(700, 213)
$Search = @(120, 126); $ClipField = @(667, 754)

# ─────────────────────────── AT-01 (F-01): never lose work silently ───────────────────────────
if (Want "AT-01") {
  $a = Fresh "at01"; $d = Join-Path $Work "d01"
  Launch -Project $a -Data $d
  $obs.t01_start = Title
  Click @IntroLead; DelKey
  $obs.t01_dirty = Title
  Ctrl 0x4E                                               # Ctrl+N while dirty
  $obs.t01_new_prompt = Dialog-Text; Shot "at01_prompt_new"
  $obs.t01_new_cancel = Press-Button "Cancel"
  $obs.t01_after_cancel = Title
  Ctrl 0x53; Start-Sleep -Milliseconds 400                # save
  Copy-Item $a "$Work\at01_saved.chipdaw" -Force
  $obs.t01_after_save = Title
  Ctrl 0x5A; $obs.t01_after_undo = Title                  # undo: differs from the saved file
  Ctrl 0x59; $obs.t01_after_redo = Title                  # redo: back to the saved state = clean
  Click @HookA; DelKey
  Ctrl 0x4E
  $obs.t01_dontsave = Press-Button "Don't Save"
  $obs.t01_after_dontsave = Title; Shot "at01_new_empty"
  Copy-Item $a "$Work\at01_after_dontsave.chipdaw" -Force
  Ctrl 0x4E                                               # clean: no prompt
  $obs.t01_clean_new_prompt = [bool](Find-Dialog 1500)
  Move-To @IntroLead; Down; Up; Down; Up; Start-Sleep -Milliseconds 400   # double-click: a new clip
  $obs.t01_untitled_dirty = Title
  Ctrl 0x4F                                               # Ctrl+O while dirty
  $obs.t01_open_prompt = Dialog-Text
  $obs.t01_open_cancel = Press-Button "Cancel"
  Ctrl 0x4F
  $obs.t01_open_dontsave = Press-Button "Don't Save"
  $obs.t01_open_filedlg = Dialog-Text                     # the native Open dialog appears
  Shot "at01_open_dialog"
  $obs.t01_open_filedlg_cancel = Press-Button "Cancel"
  $obs.t01_after_open_cancel = Title
  Invoke-Menu "File" 5                                    # File > New from MIDI File… while dirty
  $obs.t01_midi_prompt = Dialog-Text
  $obs.t01_midi_cancel = Press-Button "Cancel"
  # A .chipdaw dropped on the window (a real OLE drag).
  $b = Fresh "at01_drop"
  $t = Title
  # In the foreground: its source form must be visible to start the drag. The
  # app shows the prompt after the drop has returned.
  pwsh -NoProfile -File "$Repo\docs\ux-audit\tools\oledrag.ps1" -Title $t -File $b | Out-Null
  $obs.t01_drop_prompt = ""
  for ($i = 0; $i -lt 40 -and -not $obs.t01_drop_prompt; $i++) { Start-Sleep -Milliseconds 250; $obs.t01_drop_prompt = Dialog-Text }
  Shot "at01_prompt_drop"
  $obs.t01_drop_cancel = Press-Button "Cancel"
  $obs.t01_after_drop_cancel = Title
  AltF4                                                   # close the window
  $obs.t01_close_prompt = Dialog-Text; Shot "at01_prompt_close"
  $obs.t01_close_cancel = Press-Button "Cancel"; Start-Sleep -Milliseconds 400
  $obs.t01_alive_after_cancel = Alive
  AltF4
  $obs.t01_close_dontsave = Press-Button "Don't Save"; Start-Sleep -Seconds 2
  $obs.t01_alive_after_dontsave = Alive
  $obs.t01_marker = (Get-Content (Join-Path $d "session.lock") -ErrorAction SilentlyContinue)
  $obs.t01_recovery_file = Test-Path (Join-Path $d "recovery.cartwave")
  Launch -Data $d                                         # next start offers the unsaved work
  $obs.t01_restart_texts = (Ui-Texts) -join " | "
  Shot "at01_recovery_offer"
  Stop-App
}

# ─────────────────────────── AT-02 (F-02): debug keys only in dev mode ───────────────────────────
if (Want "AT-02") {
  $a = Fresh "at02"; $d = Join-Path $Work "d02"
  # Release build, no CARTWAVE_DEV: the project comes from the command line (CARTWAVE_PROJECT is a test hook).
  Launch -Data $d -NoDev -AppArgs @("`"$a`"")
  Move-To 1300 800                                        # empty canvas: no hover, no tooltip
  Ctrl 0x53; Start-Sleep -Milliseconds 400; Copy-Item $a "$Work\at02_before.chipdaw" -Force
  Start-Sleep -Seconds 9                                  # the "Saved" message fades first (8 s since polish 2)
  Shot "at02_before"
  foreach ($vk in 0x72, 0x73, 0x74, 0x75, 0x76) { Key $vk; Start-Sleep -Milliseconds 400 }   # F3..F7
  Start-Sleep -Milliseconds 500
  Shot "at02_after"
  # The top bar's MEM read-out is live (the process's memory), like a clock: left out.
  $obs.t02_diff_pixels = Pixel-Diff "$Work\at02_before.png" "$Work\at02_after.png" -Skip @(1330, 70, 1442, 115)
  $obs.t02_title = Title
  Ctrl 0x53; Start-Sleep -Milliseconds 400; Copy-Item $a "$Work\at02_after.chipdaw" -Force
  Stop-App
  # The same keys in dev mode do act (proves the test can see a difference).
  $a2 = Fresh "at02dev"
  Launch -Project $a2 -Data (Join-Path $Work "d02dev")
  Shot "at02_dev_before"
  Key 0x72; Start-Sleep -Milliseconds 600; Shot "at02_dev_f3"
  $obs.t02_dev_f3_diff = Pixel-Diff "$Work\at02_dev_before.png" "$Work\at02_dev_f3.png"
  Key 0x73; Start-Sleep -Milliseconds 600
  $obs.t02_dev_title = Title
  Ctrl 0x53; Start-Sleep -Milliseconds 400; Copy-Item $a2 "$Work\at02_dev_after.chipdaw" -Force
  Stop-App
}

# ─────────────────────────── AT-03 (F-03): typing never triggers shortcuts ───────────────────────────
if (Want "AT-03") {
  $a = Fresh "at03"; $d = Join-Path $Work "d03"
  Launch -Project $a -Data $d
  Shot "at03_before"
  Click @Search
  Key 0x41; Key 0x20; Key 0x42                            # "a b": Space must type, not play
  Start-Sleep -Milliseconds 1200
  KeyDown 0x11; Key 0x5A; KeyUp 0x11                      # Ctrl+Z in the field
  KeyExt 0x2E                                             # Delete in the field (deletes text, not clips)
  Start-Sleep -Milliseconds 300
  Shot "at03_search"
  $obs.t03_search_texts = (Ui-Texts) -join " | "
  $obs.t03_title_after_typing = Title
  Key 0x1B; Start-Sleep -Milliseconds 300
  # Rename a clip in place (Batch 3: Ctrl+R; the inspector's name is a
  # label now), with Ctrl+Z pressed inside the field before Enter.
  Click @IntroLead; Start-Sleep -Milliseconds 300
  Ctrl 0x52
  foreach ($c in "VERSE".ToCharArray()) { Key ([int][char]$c) }
  KeyDown 0x11; Key 0x5A; KeyUp 0x11; Start-Sleep -Milliseconds 300
  Shot "at03_rename"
  Key 0x0D; Start-Sleep -Milliseconds 300
  Click @EmptyP2
  Ctrl 0x53; Start-Sleep -Milliseconds 400; Copy-Item $a "$Work\at03_saved.chipdaw" -Force
  Stop-App
}

# ─────────────────────────── AT-04 (F-04): nothing pretends to work ───────────────────────────
if (Want "AT-04") {
  $a = Fresh "at04"; $d = Join-Path $Work "d04"
  Launch -Project $a -Data $d
  Shot "at04_before"
  Invoke-Menu "File" 8                                    # Project Settings…
  $obs.t04_clicked = Click-Text "NES (2A03)"              # Hardware profile: NES (Batch 4a labels)
  $obs.t04_prompt = Dialog-Text; Shot "at04_prompt"
  $obs.t04_switch = Press-Button "Switch anyway"         # Batch 4a: Unlock tracks / Switch anyway / Cancel
  Key 0x1B; Start-Sleep -Milliseconds 300
  Move-To 900 490; Start-Sleep -Milliseconds 500          # hover a VRC6 lane: it says why
  Shot "at04_after"
  # Screenshot y = client y + 51 (title bar 31, menu bar 20). VRC6 headers (lanes 6-8)
  # against the 2A03 headers (lanes 1-3), before and after.
  $obs.t04_luma = @(
    (Mean-Luma "$Work\at04_before.png" 250 520 520 739), (Mean-Luma "$Work\at04_before.png" 250 155 520 374),
    (Mean-Luma "$Work\at04_after.png" 250 520 520 739), (Mean-Luma "$Work\at04_after.png" 250 155 520 374))
  # A click on a VRC6 clip ("Saw Bass", lane 8) selects nothing; Delete removes nothing.
  Click 800 650; DelKey
  Ctrl 0x53; Start-Sleep -Milliseconds 400; Copy-Item $a "$Work\at04_saved.chipdaw" -Force
  Stop-App
}

# ─────────────────────────── AT-06 (F-06): menus exist and run commands ───────────────────────────
if (Want "AT-06") {
  $a = Fresh "at06"; $d = Join-Path $Work "d06"
  Launch -Project $a -Data $d
  foreach ($m in @(@("File", 0), @("Edit", 0), @("Create", 0), @("View", 0), @("Help", 0))) {
    Highlight-Menu $m[0] $m[1]; Shot ("at06_menu_" + $m[0].ToLower()); Close-Menu
  }
  Highlight-Menu "File" 2; MKey 0x27; Start-Sleep -Milliseconds 300; Shot "at06_menu_recent"; Close-Menu; Close-Menu
  $t0 = (Get-Item $a).LastWriteTimeUtc
  Click @IntroLead; DelKey
  $obs.t06_dirty = Title
  Invoke-Menu "File" 3                                    # File > Save
  $obs.t06_after_menu_save = Title
  $obs.t06_written = ((Get-Item $a).LastWriteTimeUtc -gt $t0)
  Copy-Item $a "$Work\at06_saved.chipdaw" -Force
  Stop-App
}

# ─────────────────────────── AT-15 (F-15): Settings persist ───────────────────────────
if (Want "AT-15") {
  $d = Join-Path $Work "d15"
  Launch -Data $d
  Ctrl 0xBC; Start-Sleep -Milliseconds 500                # Ctrl+,
  Shot "at15_audio"
  $obs.t15_open_texts = (Ui-Texts) -join " | "
  $obs.t15_clicked_256 = Click-Text "256 · 5.3 ms"         # Buffer size: 256
  Start-Sleep -Milliseconds 300
  foreach ($tab in "Display", "Editing", "Files", "Warnings") {
    Click-Text $tab | Out-Null; Start-Sleep -Milliseconds 300
    if ($tab -eq "Display") { $obs.t15_clicked_120 = Click-Text "120 fps"; Start-Sleep -Milliseconds 300 }
    Shot ("at15_" + $tab.ToLower())
  }
  Key 0x1B; Start-Sleep -Milliseconds 300
  $obs.t15_modal_closed_texts = (Ui-Texts) -join " | "
  Stop-App
  Copy-Item (Join-Path $d "settings.json") "$Work\at15_settings.json" -Force
  Launch -Data $d                                         # restart: the value is still there
  Ctrl 0xBC; Start-Sleep -Milliseconds 500; Shot "at15_after_restart"
  Stop-App
}

# ─────────────────────────── AT-16 (F-16/20/21): Project Settings ───────────────────────────
if (Want "AT-16") {
  $a = Fresh "at16"; $d = Join-Path $Work "d16"
  Launch -Project $a -Data $d
  Invoke-Menu "File" 8
  Shot "at16_project_settings"
  $obs.t16_clicked_profile = Click-Text "NES (2A03)"      # NES (asks: locked VRC6 tracks; Batch 4a labels)
  Press-Button "Switch anyway" | Out-Null
  Start-Sleep -Milliseconds 300
  $obs.t16_clicked_filter = Click-Text "Famicom"          # Output filter: Famicom
  Start-Sleep -Milliseconds 300
  Shot "at16_changed"
  Key 0x1B
  Ctrl 0x53; Start-Sleep -Milliseconds 400; Copy-Item $a "$Work\at16_saved.chipdaw" -Force
  Stop-App
}

# ─────────────────────────── AT-18 (F-18): Open Recent, About, shortcut sheet ───────────────────────────
if (Want "AT-18") {
  $d = Join-Path $Work "d18"
  $p1 = Fresh "first"; Launch -Project $p1 -Data $d; Ctrl 0x53; Start-Sleep -Milliseconds 300; Stop-App
  $p2 = Fresh "second"; Launch -Project $p2 -Data $d; Ctrl 0x53; Start-Sleep -Milliseconds 300; Stop-App
  Copy-Item (Join-Path $d "settings.json") "$Work\at18_settings.json" -Force
  # Plain start (the session marker from the kills offers recovery; discard it via Esc-free path).
  $gone = Fresh "gone"; Launch -Project $gone -Data $d; Ctrl 0x53; Start-Sleep -Milliseconds 300; Stop-App
  Remove-Item $gone                                       # a missing file leaves Open Recent
  Launch -Data $d
  $obs.t18_start_texts = (Ui-Texts) -join " | "
  Highlight-Menu "File" 2; MKey 0x27; Start-Sleep -Milliseconds 400; Shot "at18_open_recent"
  MKey 0x0D; Start-Sleep -Milliseconds 800                # the newest: "second"
  $obs.t18_opened = Title
  Invoke-Menu "Help" 2; Start-Sleep -Milliseconds 400     # About (after Getting Started, Batch 3)
  Shot "at18_about"
  $obs.t18_about_texts = (Ui-Texts) -join " | "
  Key 0x1B; Start-Sleep -Milliseconds 300
  Invoke-Menu "Help" 0; Start-Sleep -Milliseconds 400     # Keyboard Shortcuts
  Shot "at18_shortcuts"
  $obs.t18_sheet_texts = (Ui-Texts) -join " | "
  Key 0x1B
  Stop-App
  Copy-Item (Join-Path $d "settings.json") "$Work\at18_settings_after.chipdaw.json" -Force
}

# ─────────────────────────── AT-19 (F-19): the slim top bar ───────────────────────────
if (Want "AT-19") {
  $a = Fresh "at19"; $d = Join-Path $Work "d19"
  Launch -Project $a -Data $d
  Shot "at19_topbar"
  $t0 = (Get-Item $a).LastWriteTimeUtc
  Click @IntroLead; DelKey
  Ctrl 0x53; Start-Sleep -Milliseconds 400
  $obs.t19_saved = ((Get-Item $a).LastWriteTimeUtc -gt $t0)
  $obs.t19_title = Title
  Stop-App
}

Remove-Item Env:CARTWAVE_PROJECT, Env:CARTWAVE_DATA_DIR, Env:CARTWAVE_AUDIO, Env:CARTWAVE_DEV -ErrorAction SilentlyContinue
$obs | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $Work "obs.json") -Encoding utf8
python -I "$PSScriptRoot\check_shell.py" $Work
