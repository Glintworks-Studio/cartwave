# usage (from an elevated PowerShell): capture_server.ps1 [-Work <dir>] [-Xperf <path to xperf.exe>]
# Elevated capture helper (v2). Runs ONLY xperf kernel-trace start/stop.
# Request files in <Work>\capq: "<id>.req" containing "start|<name>" or "stop|<name>"
# (written by TraceCmd in input.ps1). Output: <Work>\etl\<name>.etl (merged, with
# image/symbol rundown). A bare "stop.req" exits. Use the same -Work as the caller.
# Defaults: -Work = $env:CHIPDAW_PERF_WORK or %TEMP%\chipdaw-perf; -Xperf = Windows Performance Toolkit.
param(
  [string]$Work = $(if ($env:CHIPDAW_PERF_WORK) { $env:CHIPDAW_PERF_WORK } else { Join-Path $env:TEMP "chipdaw-perf" }),
  [string]$Xperf = (Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\Windows Performance Toolkit\xperf.exe"))
$ErrorActionPreference = "Continue"
$Q = Join-Path $Work "capq"
$Out = Join-Path $Work "etl"
New-Item -ItemType Directory -Force $Q, $Out | Out-Null
$Host.UI.RawUI.WindowTitle = "ChipDAW capture helper v2 (elevated) - xperf start/stop only"
"ready" | Set-Content "$Q\server.ready"
while ($true) {
  if (Test-Path "$Q\stop.req") { Remove-Item "$Q\stop.req"; Remove-Item "$Q\server.ready" -ErrorAction SilentlyContinue; & $Xperf -stop 2>$null | Out-Null; break }
  foreach ($f in Get-ChildItem "$Q\*.req") {
    $line = (Get-Content $f.FullName -Raw).Trim(); Remove-Item $f.FullName
    $log = "$Q\$($f.BaseName).log"
    $parts = $line.Split('|')
    if ($parts.Count -ne 2 -or @('start', 'stop') -notcontains $parts[0] -or $parts[1] -notmatch '^[a-z0-9_\-]{1,64}$') { "rejected" | Set-Content $log; "done" | Set-Content "$Q\$($f.BaseName).done"; continue }
    $raw = Join-Path $Out "$($parts[1]).raw.etl"; $merged = Join-Path $Out "$($parts[1]).etl"
    if ($parts[0] -eq 'start') {
      & $Xperf -stop 2>$null | Out-Null
      & $Xperf -on "PROC_THREAD+LOADER+PROFILE+CSWITCH+DISPATCHER" -stackwalk "Profile+CSwitch+ReadyThread" -BufferSize 1024 -MinBuffers 256 -MaxBuffers 2048 -f $raw *> $log
    } else {
      & $Xperf -stop *> $log
      & $Xperf -merge $raw $merged *>> $log
      Remove-Item $raw -ErrorAction SilentlyContinue
    }
    "exit $LASTEXITCODE" | Add-Content $log
    "done" | Set-Content "$Q\$($f.BaseName).done"
  }
  Start-Sleep -Milliseconds 200
}
