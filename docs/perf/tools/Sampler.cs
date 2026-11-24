// usage (PowerShell): Add-Type -Path Sampler.cs; [Sampler]::Run(<pid>, <tid>, <samples>, <intervalMs>, <symbol path>)
// Returns folded stacks for the thread. Same user as the target; no elevation needed.
// Minimal non-ETW stack sampler for one thread of another process (same user).
// Suspends the thread, captures CONTEXT, walks with dbghelp, resumes.
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class Sampler {
  [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint a, bool i, uint pid);
  [DllImport("kernel32.dll")] static extern IntPtr OpenThread(uint a, bool i, uint tid);
  [DllImport("kernel32.dll")] static extern uint SuspendThread(IntPtr h);
  [DllImport("kernel32.dll")] static extern uint ResumeThread(IntPtr h);
  [DllImport("kernel32.dll")] static extern bool GetThreadContext(IntPtr h, IntPtr ctx);
  [DllImport("dbghelp.dll", CharSet = CharSet.Unicode)] static extern bool SymInitializeW(IntPtr hp, string path, bool invade);
  [DllImport("dbghelp.dll")] static extern uint SymSetOptions(uint o);
  [DllImport("dbghelp.dll")] static extern bool StackWalk64(uint machine, IntPtr hp, IntPtr ht, ref STACKFRAME64 sf, IntPtr ctx, IntPtr rd, IntPtr fta, IntPtr gmb, IntPtr tr);
  [DllImport("dbghelp.dll")] static extern IntPtr SymFunctionTableAccess64(IntPtr hp, ulong a);
  [DllImport("dbghelp.dll")] static extern ulong SymGetModuleBase64(IntPtr hp, ulong a);
  [DllImport("dbghelp.dll", CharSet = CharSet.Unicode)] static extern bool SymFromAddrW(IntPtr hp, ulong a, out ulong disp, IntPtr sym);
  [DllImport("dbghelp.dll", CharSet = CharSet.Unicode)] static extern bool SymGetModuleInfoW64(IntPtr hp, ulong a, IntPtr info);
  [DllImport("dbghelp.dll")] static extern bool SymRefreshModuleList(IntPtr hp);

  [StructLayout(LayoutKind.Sequential)] public struct ADDRESS64 { public ulong Offset; public ushort Segment; public int Mode; }
  [StructLayout(LayoutKind.Sequential)] public struct KDHELP64 { [MarshalAs(UnmanagedType.ByValArray, SizeConst = 24)] public ulong[] r; }
  [StructLayout(LayoutKind.Sequential)] public struct STACKFRAME64 {
    public ADDRESS64 AddrPC, AddrReturn, AddrFrame, AddrStack, AddrBStore; public IntPtr FuncTableEntry;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 4)] public ulong[] Params; public bool Far, Virtual;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)] public ulong[] Reserved; public KDHELP64 KdHelp;
  }

  static Dictionary<ulong, string> cache = new Dictionary<ulong, string>();
  static string Name(IntPtr hp, ulong a) {
    string n; if (cache.TryGetValue(a, out n)) return n;
    IntPtr sym = Marshal.AllocHGlobal(88 + 2 * 512);
    for (int i = 0; i < 88 + 1024; i++) Marshal.WriteByte(sym, i, 0);
    Marshal.WriteInt32(sym, 0, 88); Marshal.WriteInt32(sym, 80, 500); // SizeOfStruct, MaxNameLen
    ulong d;
    string mod = "?";
    IntPtr mi = Marshal.AllocHGlobal(4096); for (int i = 0; i < 4096; i++) Marshal.WriteByte(mi, i, 0);
    Marshal.WriteInt32(mi, 0, 3264); // sizeof IMAGEHLP_MODULEW64 (current)
    if (SymGetModuleInfoW64(hp, a, mi)) mod = Marshal.PtrToStringUni(mi + 36, 32).TrimEnd('\0'); // ModuleName at offset 44
    if (SymFromAddrW(hp, a, out d, sym)) n = mod + "!" + Marshal.PtrToStringUni(sym + 84);
    else n = mod + "!0x" + a.ToString("x");
    Marshal.FreeHGlobal(sym); Marshal.FreeHGlobal(mi);
    cache[a] = n; return n;
  }

  // Returns folded stacks "root;...;leaf count" lines.
  public static string Run(int pid, int tid, int samples, int intervalMs, string symPath) {
    IntPtr hp = OpenProcess(0x1F0FFF, false, (uint)pid);
    IntPtr ht = OpenThread(0x1F03FF, false, (uint)tid);
    SymSetOptions(0x2 | 0x4 | 0x10); // UNDNAME | DEFERRED_LOADS | LOAD_LINES
    SymInitializeW(hp, symPath, true);
    IntPtr raw = Marshal.AllocHGlobal(1232 + 16);
    IntPtr ctx = new IntPtr((raw.ToInt64() + 15) & ~15L);
    var counts = new Dictionary<string, int>();
    var rnd = new Random(1);
    for (int s = 0; s < samples; s++) {
      System.Threading.Thread.Sleep(intervalMs + rnd.Next(0, 2));
      if (SuspendThread(ht) == 0xFFFFFFFF) continue;
      var frames = new List<string>();
      try {
        for (int i = 0; i < 1232; i += 8) Marshal.WriteInt64(ctx, i, 0);
        Marshal.WriteInt32(ctx, 0x30, 0x10000B); // CONTEXT_FULL (AMD64)
        if (!GetThreadContext(ht, ctx)) continue;
        var sf = new STACKFRAME64();
        sf.Params = new ulong[4]; sf.Reserved = new ulong[3]; sf.KdHelp.r = new ulong[24];
        sf.AddrPC.Offset = (ulong)Marshal.ReadInt64(ctx, 0xF8); sf.AddrPC.Mode = 3;
        sf.AddrStack.Offset = (ulong)Marshal.ReadInt64(ctx, 0x98); sf.AddrStack.Mode = 3;
        sf.AddrFrame.Offset = (ulong)Marshal.ReadInt64(ctx, 0xA0); sf.AddrFrame.Mode = 3;
        IntPtr fta = Marshal.GetFunctionPointerForDelegate(ftaDel), gmb = Marshal.GetFunctionPointerForDelegate(gmbDel);
        for (int depth = 0; depth < 96; depth++) {
          if (!StackWalk64(0x8664, hp, ht, ref sf, ctx, IntPtr.Zero, fta, gmb, IntPtr.Zero)) break;
          if (sf.AddrPC.Offset == 0) break;
          frames.Add(sf.AddrPC.Offset.ToString());
        }
      } finally { ResumeThread(ht); }
      var names = new List<string>();
      foreach (var f in frames) names.Add(Name(hp, ulong.Parse(f)).Replace(";", ":").Replace(" ", "_"));
      names.Reverse();
      string key = string.Join(";", names);
      int c; counts.TryGetValue(key, out c); counts[key] = c + 1;
    }
    var sb = new StringBuilder();
    foreach (var kv in counts) sb.AppendLine(kv.Key + " " + kv.Value);
    return sb.ToString();
  }
  delegate IntPtr FtaDel(IntPtr hp, ulong a);
  delegate ulong GmbDel(IntPtr hp, ulong a);
  static FtaDel ftaDel = (h, a) => SymFunctionTableAccess64(h, a);
  static GmbDel gmbDel = (h, a) => SymGetModuleBase64(h, a);
}
