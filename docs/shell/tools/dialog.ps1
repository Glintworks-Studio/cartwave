# usage: . .\dialog.ps1   (dot-source; needs the app running)
# Finds the app's native dialogs (the save prompt and other task dialogs, the
# file dialogs) through UI Automation and presses their buttons by name, so a
# scenario drives the real dialog and proves it appeared.
#   Find-Dialog [-Timeout ms]          -> the dialog element (or $null)
#   Dialog-Text                        -> the dialog's text (all static text joined)
#   Press-Button "Don't Save"          -> clicks the named button; $true if found
#   Dialog-Gone [-Timeout ms]          -> $true once no dialog is open
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$global:UIA = [System.Windows.Automation.AutomationElement]
$global:Cond = [System.Windows.Automation.PropertyCondition]

function AppPid { (Get-Process -Name app | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1).Id }

function Find-Dialog([int]$Timeout = 3000) {
  $procId = AppPid
  $t0 = Get-Date
  while (((Get-Date) - $t0).TotalMilliseconds -lt $Timeout) {
    $byPid = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $procId)
    $byClass = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ClassNameProperty, "#32770")
    # A dialog with a parent window is listed under it; an unowned one at the top.
    $d = $global:UIA::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, (New-Object System.Windows.Automation.AndCondition($byPid, $byClass)))
    if (-not $d) {
      $main = $global:UIA::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $byPid)
      if ($main) { $d = $main.FindFirst([System.Windows.Automation.TreeScope]::Children, $byClass) }
    }
    if ($d) { return $d }
    Start-Sleep -Milliseconds 100
  }
  return $null
}

function Dialog-Text {
  $d = Find-Dialog 1000
  if (-not $d) { return "" }
  $c = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)
  ($d.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c) | ForEach-Object { $_.Current.Name }) -join " | "
}

function Press-Button([string]$Name) {
  $d = Find-Dialog 3000
  if (-not $d) { return $false }
  # Task-dialog buttons are "CCPushButton" panes; the file dialogs' are "Button" panes.
  $c = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::NameProperty, $Name)
  $b = $d.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c) |
    Where-Object { $_.Current.ClassName -in @("CCPushButton", "Button") -or $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button } |
    Select-Object -First 1
  if (-not $b) { return $false }
  $inv = $null
  if ($b.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$inv)) {
    $inv.Invoke()
  } else {
    # No invoke pattern: click the middle of the button.
    $r = $b.Current.BoundingRectangle
    Add-Type -Namespace DlgIn -Name U -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, int d, System.UIntPtr e);' -ErrorAction SilentlyContinue
    [DlgIn.U]::SetCursorPos([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)) | Out-Null
    Start-Sleep -Milliseconds 50
    [DlgIn.U]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40
    [DlgIn.U]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  }
  Start-Sleep -Milliseconds 400
  return $true
}

function Dialog-Gone([int]$Timeout = 3000) {
  $t0 = Get-Date
  while (((Get-Date) - $t0).TotalMilliseconds -lt $Timeout) {
    if (-not (Find-Dialog 1)) { return $true }
    Start-Sleep -Milliseconds 100
  }
  return $false
}

# Menus. vizia's accessibility tree replaces the window's default provider, so
# neither the menu bar nor its popups are visible to UI Automation. The menus
# are driven like a keyboard user would: Alt + the mnemonic opens a menu (the
# first item is highlighted), Down moves past separators, Enter invokes.
#   Invoke-Menu "File" 8   -> the 9th item of File (0-based, separators not counted)
# The item order is crates/ui/src/menus.rs `spec(false)`; screenshots show the
# highlighted item, so a wrong index is visible.
Add-Type -Namespace MenuIn -Name K -MemberDefinition '[DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, System.UIntPtr e); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);' -ErrorAction SilentlyContinue
function MKey([byte]$vk) { [MenuIn.K]::keybd_event($vk, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30; [MenuIn.K]::keybd_event($vk, 0, 2, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60 }

function Open-Menu([string]$Top) {
  $h = (Get-Process -Name app | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1).MainWindowHandle
  [MenuIn.K]::SetForegroundWindow($h) | Out-Null
  Start-Sleep -Milliseconds 100
  $vk = [byte][char]$Top.Substring(0, 1).ToUpper()
  [MenuIn.K]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30
  MKey $vk
  [MenuIn.K]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 300
}

function Highlight-Menu([string]$Top, [int]$Index) {
  Open-Menu $Top
  for ($i = 0; $i -lt $Index; $i++) { MKey 0x28 }
  Start-Sleep -Milliseconds 200
}

function Invoke-Menu([string]$Top, [int]$Index) {
  Highlight-Menu $Top $Index
  MKey 0x0D
  Start-Sleep -Milliseconds 500
}

function Close-Menu { MKey 0x1B; MKey 0x1B; Start-Sleep -Milliseconds 200 }

# The app's own labels (vizia's accessibility tree): click the one with this
# exact text (a Settings tab, a choice button's label).
function Click-Text([string]$Text) {
  $c = New-Object System.Windows.Automation.AndCondition(
    (New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)),
    (New-Object $global:Cond([System.Windows.Automation.AutomationElement]::NameProperty, $Text)))
  # The last match: an open panel is built after the rest of the window.
  $e = (Main-Window).FindAll([System.Windows.Automation.TreeScope]::Descendants, $c) | Select-Object -Last 1
  if (-not $e) { return $false }
  $r = $e.Current.BoundingRectangle
  [DlgIn.U]::SetCursorPos([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)) | Out-Null
  Start-Sleep -Milliseconds 60
  [DlgIn.U]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40
  [DlgIn.U]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 300
  return $true
}
Add-Type -Namespace DlgIn -Name U -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, int d, System.UIntPtr e);' -ErrorAction SilentlyContinue

# The app's main window in the UI Automation tree.
function Main-Window {
  $byPid = New-Object $global:Cond([System.Windows.Automation.AutomationElement]::ProcessIdProperty, (AppPid))
  $global:UIA::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $byPid)
}
