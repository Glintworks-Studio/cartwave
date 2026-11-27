# usage: render_ab.ps1 [-Work <dir>] [-Names a,b,...] [-NsfPlay C:\tools\reference\nsfplay26\nsfplay.exe]
# Batch 4c §4 (docs/ab/AB.md): renders every register log in -Work three ways, mono, 48 kHz:
#   <name>_ours.wav     our emulator replaying the log (`ab replay`, filter off = only its 5 Hz DC
#                       blocker; with the NSF player's init writes, so both start from the same state)
#   <name>_nsfplay.wav  NSFPlay 2.6 playing our NSF of the log, its filters off (LPF=0, HPF=256),
#                       no loop detection, no stop on silence, no fade; its other settings at their defaults
#   <name>_gme.wav      Game_Music_Emu (libgme, as linked in ffmpeg 8.1) playing the same NSF; its
#                       built-in equalizer and its stop after silence can't be set through ffmpeg: the
#                       equalizer is measured, and the comparison covers what it rendered (`coverage`)
# Both reference tools are separate programs installed outside the repo (C:\tools\reference);
# nothing of theirs is linked, vendored or read here. Run `ab log`, `ab patterns` and `ab nsf` first.
param([string]$Work = (Join-Path $env:TEMP "cartwave-ab"), [string]$Names = "", [string]$NsfPlay = "C:\tools\reference\nsfplay26\nsfplay.exe")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$ab = Join-Path $Repo "target\release\examples\ab.exe"
$logs = Get-ChildItem $Work -Filter *.log | Where-Object { $_.BaseName -notlike "*_nsfplay" -and (-not $Names -or ($Names.Split(",") -contains $_.BaseName)) }
foreach ($l in $logs) {
  $n = $l.BaseName
  $nsf = Join-Path $Work "$n.nsf"
  $frames = (Select-String -Path $l.FullName -Pattern "^f" | Measure-Object).Count
  $secs = [math]::Ceiling(($frames / 60.0988 + 1.0) * 1000) / 1000
  $ms = [int]($secs * 1000)
  & $ab replay $l.FullName (Join-Path $Work "${n}_ours.wav") off nsfinit | Out-Null
  $out = Join-Path $Work "${n}_nsfplay.wav"
  Remove-Item $out -ErrorAction SilentlyContinue
  $p = Start-Process $NsfPlay -WorkingDirectory (Split-Path $NsfPlay) -ArgumentList "-LPF=0", "-HPF=256", "-NCH=1", "-RATE=48000", "-AUTO_DETECT=0", "-AUTO_STOP=0", "-FADE_TIME=0", "`"$nsf`"", "`"$out`"", "1", "$ms" -PassThru -Wait -RedirectStandardOutput (Join-Path $Work "${n}_nsfplay.txt")
  $gme = Join-Path $Work "${n}_gme.wav"
  ffmpeg -hide_banner -loglevel error -y -sample_rate 48000 -i $nsf -t $secs -ac 1 -c:a pcm_s16le $gme
  "{0}: {1} frames, {2} s; nsfplay exit {3}, {4} bytes; gme {5} bytes" -f $n, $frames, $secs, $p.ExitCode, (Get-Item $out -ErrorAction SilentlyContinue).Length, (Get-Item $gme -ErrorAction SilentlyContinue).Length
}
