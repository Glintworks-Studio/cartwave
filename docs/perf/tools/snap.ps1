# usage: snap.ps1 -Out <file.png> [-Focus] [-Repo <repo>]
# Static, deterministic frame: app freshly launched, stopped, cursor outside the window.
# -Focus focuses the search field first (to show the glow). Default -Repo = this script's repo.
param([string]$Out, [switch]$Focus,
  [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path)
$env:CHIPDAW_DEV = "1"   # F-02: test hooks (CHIPDAW_PROJECT, _STRESS, …) and F3-F7 need dev mode
$Out = [IO.Path]::GetFullPath($Out)
Set-Location $Repo
Get-Process -Name app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process ".\target\release\app.exe" | Out-Null; Start-Sleep 3
. "$PSScriptRoot\input.ps1"
if ($Focus) { Move-To 746 790; Down; Up }   # focus the search field (glow)
Move-To 1500 1000; Start-Sleep -Milliseconds 1200
& $Shot -Out $Out | Out-Null
Get-Process -Name app | Stop-Process -Force
