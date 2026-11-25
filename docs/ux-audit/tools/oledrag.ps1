# oledrag.ps1: a real OLE file drag (as from Explorer) onto a window, without a human.
#
#   pwsh -File oledrag.ps1 -Title DndWinitProbe -File C:\path\song.mid [-Mode drop|leave|escape]
#
# How: a small WinForms "source" window appears left of the target. A background thread
# presses the left button on it with SendInput; the form's MouseDown starts
# Control.DoDragDrop with a FileDrop DataObject (the same CF_HDROP format Explorer offers).
# While DoDragDrop's modal loop runs, the background thread moves the cursor
# (SetCursorPos + a zero relative SendInput move, so the drag loop sees a mouse message)
# through the target's client area in steps, pauses, then:
#   drop    releases the button inside the target          -> Drop
#   leave   moves back out over the source, then releases   -> DragLeave
#   escape  presses Esc inside the target                   -> DragLeave (cancelled drag)
# Every step is printed with wall-clock time and screen/client coordinates, to line up
# with the probe's own log. The process is per-monitor DPI aware (physical pixels).
param(
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][string]$File,
    [ValidateSet('drop', 'leave', 'escape')][string]$Mode = 'drop',
    [int]$StepMs = 60
)

Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing, System.Drawing.Primitives, System.Collections.Specialized, System.ComponentModel.Primitives, System.Threading.Thread, System.Runtime.InteropServices, System.Console, System.Runtime -TypeDefinition @'
using System;
using System.Collections.Specialized;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;

public static class OleDrag {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr extra; }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr extra; }
    [StructLayout(LayoutKind.Explicit)] public struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public INPUTUNION u; }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr v);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);

    static void Log(string s) { Console.WriteLine(DateTime.Now.ToString("HH:mm:ss.fff") + " oledrag: " + s); }
    static void Mouse(uint flags) {
        var i = new INPUT[1]; i[0].type = 0; i[0].u.mi.dwFlags = flags;
        SendInput(1, i, Marshal.SizeOf(typeof(INPUT)));
    }
    static void Key(ushort vk, bool up) {
        var i = new INPUT[1]; i[0].type = 1; i[0].u.ki.wVk = vk; i[0].u.ki.dwFlags = up ? 2u : 0u;
        SendInput(1, i, Marshal.SizeOf(typeof(INPUT)));
    }
    static void MoveTo(int x, int y) { SetCursorPos(x, y); Mouse(0x0001); }

    public static int Run(string title, string file, string mode, int stepMs) {
        SetProcessDpiAwarenessContext(new IntPtr(-4));
        IntPtr target = FindWindowW(null, title);
        if (target == IntPtr.Zero) { Log("target window not found: " + title); return 1; }
        RECT cr; GetClientRect(target, out cr);
        var origin = new POINT(); ClientToScreen(target, ref origin);
        int w = cr.R, h = cr.B;
        Log(string.Format("target HWND=0x{0:X} client {1}x{2} at screen ({3},{4})", target.ToInt64(), w, h, origin.X, origin.Y));
        int result = 0;
        // The drop target is whatever window is under the cursor: make the target topmost
        // for the duration, so no other window (a terminal, a running ChipDAW) gets the drop.
        SetWindowPos(target, new IntPtr(-1), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010);
        var t = new Thread(() => {
            var form = new Form();
            form.Text = "OleDragSource"; form.FormBorderStyle = FormBorderStyle.FixedToolWindow;
            form.StartPosition = FormStartPosition.Manual; form.TopMost = true;
            form.SetBounds(Math.Max(0, origin.X - 220), origin.Y + 20, 180, 120);
            bool started = false;
            form.MouseDown += (s, e) => {
                started = true;
                Log("source MouseDown -> DoDragDrop(FileDrop: " + file + ")");
                var data = new DataObject();
                var files = new StringCollection(); files.Add(file);
                data.SetFileDropList(files);
                var eff = form.DoDragDrop(data, DragDropEffects.Copy | DragDropEffects.Link);
                Log("DoDragDrop returned " + eff);
                form.BeginInvoke(new Action(() => form.Close()));
            };
            form.Shown += (s, e) => {
                SetForegroundWindow(form.Handle);
                var c = form.PointToScreen(new System.Drawing.Point(form.ClientSize.Width / 2, form.ClientSize.Height / 2));
                var mover = new Thread(() => {
                    Thread.Sleep(400);
                    MoveTo(c.X, c.Y); Thread.Sleep(100);
                    Log("left button down on the source at screen (" + c.X + "," + c.Y + ")");
                    Mouse(0x0002); Thread.Sleep(300);
                    // Nudge past the drag threshold, still on the source.
                    MoveTo(c.X + 8, c.Y + 8); Thread.Sleep(stepMs);
                    // Path through the target: left edge -> right, then down.
                    int y0 = origin.Y + h / 3, y1 = origin.Y + 2 * h / 3;
                    for (int i = 0; i <= 10; i++) {
                        int x = origin.X + 10 + (w - 20) * i / 10;
                        MoveTo(x, y0);
                        Log(string.Format("move screen ({0},{1}) = client ({2},{3})", x, y0, x - origin.X, y0 - origin.Y));
                        Thread.Sleep(stepMs);
                    }
                    for (int i = 1; i <= 4; i++) {
                        int x = origin.X + w - 10 - (w / 2) * i / 4; int y = y0 + (y1 - y0) * i / 4;
                        MoveTo(x, y);
                        Log(string.Format("move screen ({0},{1}) = client ({2},{3})", x, y, x - origin.X, y - origin.Y));
                        Thread.Sleep(stepMs);
                    }
                    Log("hold still 500 ms"); Thread.Sleep(500);
                    if (mode == "drop") {
                        Log("left button up inside the target"); Mouse(0x0004);
                    } else if (mode == "escape") {
                        Log("Esc inside the target"); Key(0x1B, false); Thread.Sleep(30); Key(0x1B, true);
                        Thread.Sleep(200); Mouse(0x0004);
                    } else {
                        MoveTo(c.X, c.Y); Log("moved back onto the source; left button up"); Thread.Sleep(stepMs);
                        Mouse(0x0004);
                    }
                    // Watchdog: if the press never reached the source, no drag ran; say so and quit.
                    Thread.Sleep(1500);
                    if (!started) {
                        Log("NO DRAG: the button press did not reach the source window; retry");
                        form.BeginInvoke(new Action(() => form.Close()));
                    }
                });
                mover.IsBackground = true; mover.Start();
            };
            Application.Run(form);
        });
        t.SetApartmentState(ApartmentState.STA);
        t.Start(); t.Join();
        SetWindowPos(target, new IntPtr(-2), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010);
        return result;
    }
}
'@

exit [OleDrag]::Run($Title, (Resolve-Path $File).Path, $Mode, $StepMs)
