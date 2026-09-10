# Send mouse wheel via user32 mouse_event (bypasses cursor-position dependency).
param([int]$Notches = 1)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Wheel {
  [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, int dwData, UIntPtr dwExtraInfo);
}
"@
# MOUSEEVENTF_WHEEL = 0x0800, one notch = WHEEL_DELTA = 120; positive = scroll up
$dir = if ($Notches -ge 0) { 1 } else { -1 }
for ($i = 0; $i -lt [Math]::Abs($Notches); $i++) {
  [Win32Wheel]::mouse_event(0x0800, 0, 0, 120 * $dir, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
}
Write-Output "WHEEL $Notches notches"
