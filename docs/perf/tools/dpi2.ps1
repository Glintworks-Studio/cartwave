# usage: dpi2.ps1 [-Percent <scale>]   (0 = only report)
# Reports each display path's refresh rate and scaling via QueryDisplayConfig;
# with -Percent it sets that scaling (100, 125, 150, ...) on each active path.
param([int]$Percent = 0)  # 0 = just report
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Dpi {
  [StructLayout(LayoutKind.Sequential)] public struct LUID { public uint Low; public int High; }
  [StructLayout(LayoutKind.Sequential)] public struct PATH_SOURCE { public LUID adapterId; public uint id; public uint modeInfoIdx; public uint statusFlags; }
  [StructLayout(LayoutKind.Sequential)] public struct PATH_TARGET { public LUID adapterId; public uint id; public uint modeInfoIdx; public uint outputTechnology; public uint rotation; public uint scaling; public uint refN; public uint refD; public uint scanLineOrdering; public int targetAvailable; public uint statusFlags; }
  [StructLayout(LayoutKind.Sequential)] public struct PATH_INFO { public PATH_SOURCE sourceInfo; public PATH_TARGET targetInfo; public uint flags; }
  [StructLayout(LayoutKind.Sequential, Size = 64)] public struct MODE_INFO { public uint infoType; }
  [StructLayout(LayoutKind.Sequential)] public struct HEADER { public int type; public uint size; public LUID adapterId; public uint id; }
  [StructLayout(LayoutKind.Sequential)] public struct GET_DPI { public HEADER header; public int minRel; public int curRel; public int maxRel; }
  [StructLayout(LayoutKind.Sequential)] public struct SET_DPI { public HEADER header; public int rel; }
  [DllImport("user32.dll")] public static extern int GetDisplayConfigBufferSizes(uint flags, out uint np, out uint nm);
  [DllImport("user32.dll")] public static extern int QueryDisplayConfig(uint flags, ref uint np, [Out] PATH_INFO[] p, ref uint nm, [Out] MODE_INFO[] m, IntPtr topo);
  [DllImport("user32.dll")] public static extern int DisplayConfigGetDeviceInfo(ref GET_DPI g);
  [DllImport("user32.dll")] public static extern int DisplayConfigSetDeviceInfo(ref SET_DPI s);
  static readonly int[] Steps = { 100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500 };
  public static string Run(int percent) {
    uint np, nm; GetDisplayConfigBufferSizes(2, out np, out nm);
    var paths = new PATH_INFO[np]; var modes = new MODE_INFO[nm];
    int r = QueryDisplayConfig(2, ref np, paths, ref nm, modes, IntPtr.Zero);
    if (r != 0) return "QueryDisplayConfig failed " + r;
    string log = "";
    for (int i = 0; i < np; i++) {
      var g = new GET_DPI(); g.header.type = -3; g.header.size = (uint)Marshal.SizeOf(typeof(GET_DPI));
      g.header.adapterId = paths[i].sourceInfo.adapterId; g.header.id = paths[i].sourceInfo.id;
      if (DisplayConfigGetDeviceInfo(ref g) != 0) { log += "path " + i + ": get failed\n"; continue; }
      int rec = -g.minRel; int cur = rec + g.curRel;
      log += "path " + i + ": adapter LUID " + paths[i].targetInfo.adapterId.High + ":" + paths[i].targetInfo.adapterId.Low + " target refresh " + paths[i].targetInfo.refN + "/" + paths[i].targetInfo.refD + " = " + (paths[i].targetInfo.refD == 0 ? 0 : (double)paths[i].targetInfo.refN / paths[i].targetInfo.refD) + " Hz; current " + Steps[cur] + "% (recommended " + Steps[rec] + "%)\n";
      if (percent > 0) {
        int idx = Array.IndexOf(Steps, percent);
        if (idx < 0) return log + "unsupported percent";
        var s = new SET_DPI(); s.header.type = -4; s.header.size = (uint)Marshal.SizeOf(typeof(SET_DPI));
        s.header.adapterId = g.header.adapterId; s.header.id = g.header.id; s.rel = idx - rec;
        log += "  set " + percent + "% -> " + DisplayConfigSetDeviceInfo(ref s) + "\n";
      }
    }
    return log;
  }
}
"@
[Dpi]::Run($Percent)

