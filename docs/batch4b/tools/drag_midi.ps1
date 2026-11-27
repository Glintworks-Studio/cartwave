# usage: drag_midi.ps1 -File <path> -Points "x,y;x,y;..." [-StepMs 120] [-Cancel] [-ShotAt <n> -Shot <png>]
# Batch 4b §6.1: a real OS file drag (OLE DoDragDrop, as Explorer does) of -File onto whatever
# window is under the points (screen px). A small helper window starts the drag with the left
# button held; a thread moves the cursor through -Points, one step every -StepMs, then releases
# (a drop) or presses Esc first (-Cancel: the drag leaves without a drop). With -ShotAt n, a
# screenshot of the primary screen is saved after the n-th point (the preview mid-drag).
param([string]$File, [string]$Points, [int]$StepMs = 120, [switch]$Cancel, [int]$ShotAt = -1, [string]$Shot = "")
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing, System.Drawing.Primitives, System.ComponentModel.Primitives, System.Windows.Forms.Primitives, System.Threading.Thread, System.Threading, System.Runtime.InteropServices, System.Collections.Specialized, System.Drawing.Common, System.Private.Windows.Core, System.Private.Windows.GdiPlus @"
using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;
public static class OleDrag {
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
    [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
    const uint DOWN = 2, UP = 4;
    public static string Run(string file, int[] xs, int[] ys, int stepMs, bool cancel, int shotAt, string shot) {
        SetProcessDPIAware();
        var log = new System.Text.StringBuilder();
        var f = new Form { Width = 40, Height = 40, TopMost = true, StartPosition = FormStartPosition.Manual, Location = new Point(xs[0] - 20, ys[0] - 20), FormBorderStyle = FormBorderStyle.None, ShowInTaskbar = false };
        bool started = false;
        f.MouseDown += (s, e) => {
            if (started) return;
            started = true;
            var data = new DataObject(DataFormats.FileDrop, new string[] { file });
            var r = f.DoDragDrop(data, DragDropEffects.Copy | DragDropEffects.Move | DragDropEffects.Link);
            log.AppendLine("result " + r);
            f.BeginInvoke((Action)(() => f.Close()));
        };
        f.Shown += (s, e) => {
            var t = new Thread(() => {
                Thread.Sleep(300);
                SetCursorPos(xs[0], ys[0]);
                Thread.Sleep(100);
                mouse_event(DOWN, 0, 0, 0, UIntPtr.Zero);
                Thread.Sleep(300);
                for (int i = 1; i < xs.Length; i++) {
                    // Small moves on the way so the drag notices each one.
                    for (int k = 1; k <= 4; k++) {
                        int x = xs[i - 1] + (xs[i] - xs[i - 1]) * k / 4, y = ys[i - 1] + (ys[i] - ys[i - 1]) * k / 4;
                        SetCursorPos(x, y);
                        mouse_event(1, 0, 0, 0, UIntPtr.Zero);
                        Thread.Sleep(stepMs / 4);
                    }
                    log.AppendLine("at " + xs[i] + "," + ys[i]);
                    if (i == shotAt && shot.Length > 0) {
                        Thread.Sleep(300);
                        var b = Screen.PrimaryScreen.Bounds;
                        using (var bmp = new Bitmap(b.Width, b.Height)) {
                            using (var g = Graphics.FromImage(bmp)) g.CopyFromScreen(b.Location, Point.Empty, b.Size);
                            bmp.Save(shot);
                        }
                    }
                }
                if (cancel) { keybd_event(0x1B, 0, 0, UIntPtr.Zero); keybd_event(0x1B, 0, 2, UIntPtr.Zero); Thread.Sleep(100); }
                mouse_event(UP, 0, 0, 0, UIntPtr.Zero);
            });
            t.IsBackground = true;
            t.Start();
        };
        Application.Run(f);
        return log.ToString();
    }
}
"@
$pts = $Points -split ";" | Where-Object { $_ } | ForEach-Object { $a = $_ -split ","; [pscustomobject]@{ x = [int]$a[0]; y = [int]$a[1] } }
[OleDrag]::Run((Resolve-Path $File).Path, [int[]]($pts.x), [int[]]($pts.y), $StepMs, [bool]$Cancel, $ShotAt, $Shot)
