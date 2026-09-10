# Send relative mouse motion via user32 mouse_event (bypasses synthetic-message interception).
param(
    [int]$Dx = 0,
    [int]$Dy = 0
)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Mouse {
  [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, UIntPtr dwExtraInfo);
}
"@
# MOUSEEVENTF_MOVE = 0x0001 (relative motion)
[Win32Mouse]::mouse_event(0x0001, $Dx, $Dy, 0, [UIntPtr]::Zero)
Write-Output "MOVED $Dx $Dy"
