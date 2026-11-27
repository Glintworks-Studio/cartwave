# usage: compare_ab.ps1 [-Work <dir>] [-Out docs\ab\results.tsv] [-Pack docs\audio\ab]
# Batch 4c §4: runs `ab_compare` on every render triple render_ab.ps1 made (ours, NSFPlay, gme):
# the test patterns with their segments, then each song channel by channel, then the full mixes.
# Appends every measurement to -Out (TSV: name, segment, metric, NSFPlay, gme) and writes the
# listening pack (aligned, level-matched A/B/C WAVs) for the patterns and the songs into -Pack.
param([string]$Work = (Join-Path $env:TEMP "cartwave-ab"), [string]$Out = "", [string]$Pack = "")
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
if (-not $Out) { $Out = Join-Path $Repo "docs\ab\results.tsv" }
if (-not $Pack) { $Pack = Join-Path $Repo "docs\audio\ab" }
$cmp = Join-Path $Repo "target\release\examples\ab_compare.exe"
"name`tsegment`tmetric`tnsfplay`tgme" | Set-Content $Out
$patterns = "pulse1", "pulse2", "envelope", "sweep", "triangle", "noise_long", "noise_short", "dpcm", "mixer", "vrc6"
$chan = foreach ($s in "demo", "showcase") { foreach ($g in "p1", "p2", "tri", "noise", "dmc", "v1", "v2", "saw") { "$s-$g" } }
foreach ($n in @($patterns) + @($chan) + @("demo", "showcase")) {
  $seg = Join-Path $Work "$n.seg"
  if (-not (Test-Path $seg)) { $seg = "-" }
  $args2 = @($n, $seg, (Join-Path $Work "${n}_ours.wav"), (Join-Path $Work "${n}_nsfplay.wav"), (Join-Path $Work "${n}_gme.wav"), "--tsv", $Out)
  # The pack: every pattern, and the two songs (their channels are in the TSV only).
  if ($n -notlike "*-*") { $args2 += @("--pack", $Pack) }
  & $cmp @args2 | Out-Null
  "$n done"
}
