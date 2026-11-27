# usage: matrix.ps1 [-Repo <dir>] [-Work <dir>] [-AppExe <exe>] [-Tests A,B,C,D,E,G,H] [-Buffers 128,256,512] [-LongSecs 600] [-Gain 0.3]
# Runs the ENGINE.md §5 stress matrix against the release build. Each run
# writes <Work>\<test>_<buffer>.csv (one line per second + a summary at the
# end, see crates/ui/src/testing.rs). Ground truth for dropouts is WASAPI
# loopback of a 19 kHz pilot tone (CARTWAVE_PILOT=1).
param(
    [string]$Repo = (Resolve-Path "$PSScriptRoot\..\..\.."),
    [string]$Work = $(if ($env:CARTWAVE_PERF_WORK) { $env:CARTWAVE_PERF_WORK } else { "$env:TEMP\chipdaw-engine" }),
    [string[]]$Tests = @('A', 'B', 'C', 'D', 'G', 'E'),
    [string[]]$Buffers = @('128', '256', '512'),
    [int]$LongSecs = 600,
    [double]$Gain = 0.3,
    # A frozen copy of the build, so the repo can be rebuilt while the matrix runs.
    [string]$AppExe = ''
)
$env:CARTWAVE_DEV = "1"   # F-02: test hooks (CARTWAVE_PROJECT, _STRESS, …) and F3-F7 need dev mode
$ErrorActionPreference = 'Stop'
# `pwsh -File` passes "A,B" as one string: split comma lists.
$Tests = $Tests | ForEach-Object { $_ -split ',' } | Where-Object { $_ }
$Buffers = $Buffers | ForEach-Object { $_ -split ',' } | Where-Object { $_ }
New-Item -ItemType Directory -Force $Work | Out-Null
$app = if ($AppExe) { $AppExe } else { Join-Path $Repo 'target\release\cartwave.exe' }

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class ChipdawWin {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
}
"@

function Run-App([string]$name, [hashtable]$envs, [int]$secs, [scriptblock]$during) {
    $log = Join-Path $Work "$name.csv"
    $base = @{
        CARTWAVE_AUDIOLOG = $log; CARTWAVE_QUIT_AFTER = "$secs"; CARTWAVE_PILOT = '1';
        CARTWAVE_GAIN = "$Gain"
    }
    $all = $base + $envs
    foreach ($k in $all.Keys) { Set-Item "Env:$k" $all[$k] }
    $p = Start-Process -FilePath $app -PassThru -WorkingDirectory $Repo
    try {
        if ($during) { & $during $p }
        if (-not $p.WaitForExit(($secs + 60) * 1000)) { $p.Kill(); "TIMEOUT $name" }
    } finally {
        foreach ($k in $all.Keys) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
    }
    "$name exit=$($p.ExitCode)"
    Get-Content $log | Where-Object { $_ -like '#*' }
}

function Wait-Window($p) {
    for ($i = 0; $i -lt 100; $i++) {
        $p.Refresh()
        if ($p.MainWindowHandle -ne 0) { return $p.MainWindowHandle }
        Start-Sleep -Milliseconds 100
    }
    throw "no window"
}

# D: resize (incl. a live-drag-like sweep), minimise/restore, while playing.
$churn = {
    param($p)
    $h = Wait-Window $p
    $t0 = Get-Date
    $sizes = @(@(1440, 900), @(1100, 700), @(1700, 1000), @(1300, 820))
    $i = 0
    while (((Get-Date) - $t0).TotalSeconds -lt 110) {
        $s = $sizes[$i % $sizes.Count]
        [ChipdawWin]::SetWindowPos($h, [IntPtr]::Zero, 40, 40, $s[0], $s[1], 0x0004) | Out-Null
        Start-Sleep -Milliseconds 1500
        # Sweep like a mouse resize: 30 steps, 16 ms apart.
        for ($k = 0; $k -lt 30; $k++) {
            [ChipdawWin]::SetWindowPos($h, [IntPtr]::Zero, 40, 40, 1000 + 20 * $k, 650 + 10 * $k, 0x0004) | Out-Null
            Start-Sleep -Milliseconds 16
        }
        [ChipdawWin]::ShowWindow($h, 6) | Out-Null   # SW_MINIMIZE
        Start-Sleep -Milliseconds 2000
        [ChipdawWin]::ShowWindow($h, 9) | Out-Null   # SW_RESTORE
        Start-Sleep -Milliseconds 1500
        $i++
    }
}

# E: switch the default output away and back while playing.
$devSwitch = {
    param($p)
    Wait-Window $p | Out-Null
    Start-Sleep -Seconds 15
    pwsh -NoProfile -File "$PSScriptRoot\set_default_audio.ps1" -Name 'VX3211'
    Start-Sleep -Seconds 15
    pwsh -NoProfile -File "$PSScriptRoot\set_default_audio.ps1" -Name 'MOTU'
}

foreach ($t in $Tests) {
    switch ($t) {
        'A' { foreach ($b in $Buffers) { Run-App "A_$b" @{ CARTWAVE_BUFFER = "$b"; CARTWAVE_AUTOPLAY = '1' } $LongSecs $null } }
        'B' { foreach ($b in $Buffers) { Run-App "B_$b" @{ CARTWAVE_BUFFER = "$b"; CARTWAVE_STRESS = '1'; CARTWAVE_AUTO_EDIT = '1' } $LongSecs $null } }
        'C' { foreach ($b in $Buffers) { Run-App "C_$b" @{ CARTWAVE_BUFFER = "$b"; CARTWAVE_AUTOPLAY = '1'; CARTWAVE_UI_STALL = '1' } 120 $null } }
        'D' { foreach ($b in $Buffers) { Run-App "D_$b" @{ CARTWAVE_BUFFER = "$b"; CARTWAVE_AVSYNC = '1' } 120 $churn } }
        'G' { foreach ($b in $Buffers) { Run-App "G_$b" @{ CARTWAVE_BUFFER = "$b"; CARTWAVE_AVSYNC = '1' } 120 $null } }
        # H: the macro worst case (every lane runs all four macros + vibrato, long notes).
        'H' { Run-App "H" @{ CARTWAVE_AUTOPLAY = '1'; CARTWAVE_SONG = 'heavy' } $LongSecs $null }
        'E' { Run-App "E" @{ CARTWAVE_AUTOPLAY = '1' } 50 $devSwitch }
    }
}
