# usage: set_default_audio.ps1 -List | -Name "<endpoint friendly name substring>"
# Lists render endpoints, or makes one the Windows default output (all roles),
# via the undocumented-but-stable IPolicyConfig COM interface (what the Sound
# control panel uses). Used by stress test E (ENGINE.md §5). Restore afterwards.
param([switch]$List, [string]$Name)

Add-Type @"
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("f8679f50-850a-41cf-9c72-430f290290c8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig {
    int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr fmt);
    int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, int def, IntPtr fmt);
    int ResetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id);
    int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr a, IntPtr b);
    int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, int def, IntPtr a, IntPtr b);
    int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr p);
    int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr m);
    int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr m);
    int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr key, IntPtr v);
    int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr key, IntPtr v);
    int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string id, int role);
    int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string id, int visible);
}

[ComImport, Guid("870af99c-171d-4f9e-af0d-e63df40c2bc9")]
class CPolicyConfigClient { }

public static class ChipdawAudioDefault {
    public static void Set(string id) {
        var p = (IPolicyConfig)new CPolicyConfigClient();
        for (int role = 0; role < 3; role++) {
            Marshal.ThrowExceptionForHR(p.SetDefaultEndpoint(id, role));
        }
    }
}
"@

# Render endpoints are SWD\MMDEVAPI\{0.0.0.00000000}.{guid}.
$eps = Get-PnpDevice -Class AudioEndpoint -Status OK |
    Where-Object { $_.InstanceId -like 'SWD\MMDEVAPI\{0.0.0.*' } |
    ForEach-Object { [pscustomobject]@{ Name = $_.FriendlyName; Id = $_.InstanceId.Substring(13) } }
if ($List) { $eps | Format-Table -AutoSize; return }
$ep = $eps | Where-Object { $_.Name -like "*$Name*" } | Select-Object -First 1
if (-not $ep) { throw "no render endpoint matching '$Name'" }
[ChipdawAudioDefault]::Set($ep.Id)
"default output -> $($ep.Name)"
