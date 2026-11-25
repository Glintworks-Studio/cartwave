# uxdrive.ps1: synthetic input for the UX-audit probes (Windows only).
#
# Usage:
#   pwsh -File uxdrive.ps1 -Title MenuProbe -Do "activate; keys ctrl+s; wait 300; keys alt+f; wait 1000; keys esc esc"
#   pwsh -File uxdrive.ps1 -Title MenuProbe -Do "shot C:\path\menu.png"
#
# Steps (separated by ';'):
#   activate              click the centre of the window's client area (gives it focus)
#   click X Y             left click at client coordinates (physical px)
#   keys K [K ...]        each K is a chord like ctrl+s, ctrl+shift+s, alt+f, esc, down, enter, f
#   wait MS               sleep
#   shot PATH             screenshot of the window rectangle (PNG)
#   close                 post WM_CLOSE to the window
# The process is made per-monitor DPI aware so coordinates are physical pixels.
param(
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][string]$Do
)

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class UxDrive {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr extra; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr extra; }
    [StructLayout(LayoutKind.Explicit)] public struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public INPUTUNION u; }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
    public static void Dpi() { SetProcessDpiAwarenessContext(new IntPtr(-4)); }
    public static void Mouse(uint flags) {
        var i = new INPUT[1]; i[0].type = 0; i[0].u.mi.dwFlags = flags;
        SendInput(1, i, Marshal.SizeOf(typeof(INPUT)));
    }
    public static void Key(ushort vk, bool up) {
        var i = new INPUT[1]; i[0].type = 1; i[0].u.ki.wVk = vk; i[0].u.ki.dwFlags = up ? 2u : 0u;
        SendInput(1, i, Marshal.SizeOf(typeof(INPUT)));
    }
    public static void ClickClient(IntPtr h, int x, int y) {
        var p = new POINT { X = x, Y = y }; ClientToScreen(h, ref p);
        SetCursorPos(p.X, p.Y); System.Threading.Thread.Sleep(30);
        Mouse(0x0002); System.Threading.Thread.Sleep(30); Mouse(0x0004);
    }
}
'@

[UxDrive]::Dpi() | Out-Null
$h = [UxDrive]::FindWindowW([NullString]::Value, $Title)
if ($h -eq [IntPtr]::Zero) { Write-Error "window '$Title' not found"; exit 1 }

$vk = @{ ctrl = 0x11; shift = 0x10; alt = 0x12; esc = 0x1B; enter = 0x0D; down = 0x28; up = 0x26; left = 0x25; right = 0x27; tab = 0x09; space = 0x20; f10 = 0x79; delete = 0x2E }

function Send-Chord([string]$chord) {
    $parts = $chord.ToLower().Split('+')
    $codes = foreach ($p in $parts) {
        if ($vk.ContainsKey($p)) { $vk[$p] } elseif ($p.Length -eq 1) { [int][char]$p.ToUpper() } else { throw "unknown key $p" }
    }
    foreach ($c in $codes) { [UxDrive]::Key([uint16]$c, $false); Start-Sleep -Milliseconds 20 }
    [array]::Reverse($codes)
    foreach ($c in $codes) { [UxDrive]::Key([uint16]$c, $true); Start-Sleep -Milliseconds 20 }
    Start-Sleep -Milliseconds 60
}

foreach ($step in $Do.Split(';')) {
    $w = $step.Trim().Split(' ', [StringSplitOptions]::RemoveEmptyEntries)
    if ($w.Count -eq 0) { continue }
    $t = [DateTime]::Now.ToString('HH:mm:ss.fff')
    Write-Output "$t uxdrive: $($step.Trim())"
    switch ($w[0]) {
        'activate' {
            $r = New-Object UxDrive+RECT; [UxDrive]::GetClientRect($h, [ref]$r) | Out-Null
            [UxDrive]::ClickClient($h, [int](($r.R) / 2), [int](($r.B) / 2))
        }
        'click' { [UxDrive]::ClickClient($h, [int]$w[1], [int]$w[2]) }
        'keys' { foreach ($k in $w[1..($w.Count - 1)]) { Send-Chord $k } }
        'wait' { Start-Sleep -Milliseconds ([int]$w[1]) }
        'shot' {
            $r = New-Object UxDrive+RECT; [UxDrive]::GetWindowRect($h, [ref]$r) | Out-Null
            $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
            $bmp.Save($w[1], [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
        }
        'close' { [UxDrive]::PostMessageW($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null }
        default { throw "unknown step $($w[0])" }
    }
}
