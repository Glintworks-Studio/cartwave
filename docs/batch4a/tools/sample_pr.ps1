# usage: sample_pr.ps1 -Repo <repo> -Out <folded> [-Samples 3000] [-NoteY 623]
# Batch 4a gate 6 attribution for the piano-roll bench (bench_pr.ps1's scene: piano_bench.chipdaw,
# its clip open at 32 px per bar, playing): samples the UI thread while the handle note is dragged
# the way bench_pr drags it.
param([string]$Repo, [string]$Out, [int]$Samples = 3000, [int]$NoteY = 623, [switch]$NoDrag)
# -NoDrag: sample while it plays, without the drag.
$Tools = Join-Path $PSScriptRoot "..\..\perf\tools"
Get-Process -Name cartwave -ErrorAction SilentlyContinue | Stop-Process -Force
$env:CARTWAVE_DEV = "1"; $env:CARTWAVE_PROJECT = "$env:TEMP\chipdaw-perf-dp\piano_bench.chipdaw"
$env:CARTWAVE_OPEN_CLIP = "1"; $env:CARTWAVE_PIANO_ZOOM = "32"; $env:CARTWAVE_DATA_DIR = "$env:TEMP\chipdaw-sample-pr"
Set-Location $Repo
$p = Start-Process ".\target\release\cartwave.exe" -PassThru
Start-Sleep -Seconds 3
$p.Refresh(); $global:AppH = $p.MainWindowHandle
. (Join-Path $Tools "input.ps1")
Move-To 700 300; Key 0x20; Start-Sleep -Seconds 2
$tid = ($p.Threads | Sort-Object StartTime | Select-Object -First 1).Id
$job = Start-Job -ScriptBlock { param($t, $ap, $at, $n, $sy) Add-Type -Path $t; [Sampler]::Run($ap, $at, $n, 3, $sy) } -ArgumentList (Join-Path $Tools "Sampler.cs"), $p.Id, $tid, $Samples, (Join-Path $Repo "target\release")
if (-not $NoDrag) { Move-To 88 $NoteY; Down }
$i = 0
while ($job.State -eq "Running") { if ($NoDrag) { Start-Sleep -Milliseconds 100; continue }; Move-To (88 + [int](12 * [Math]::Sin($i / 12.0))) ($NoteY + [int](48 * [Math]::Sin($i / 30.0))); $i++; Start-Sleep -Milliseconds 8 }
if (-not $NoDrag) { Up }
$folded = Receive-Job $job | Out-String
Stop-Process -Id $p.Id -Force
Set-Content -Path $Out -Value $folded -Encoding utf8
"tid $tid; lines $(($folded -split "`n").Count)"
