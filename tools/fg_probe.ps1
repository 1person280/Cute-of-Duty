# Report which process owns the foreground window (lock-state probe).
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class FG {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hwnd, StringBuilder sb, int max);
}
"@
$h = [FG]::GetForegroundWindow()
$pid2 = 0
[FG]::GetWindowThreadProcessId($h, [ref]$pid2) | Out-Null
$proc = Get-Process -Id $pid2 -ErrorAction SilentlyContinue
$sb = New-Object System.Text.StringBuilder 256
[FG]::GetWindowText($h, $sb, 256) | Out-Null
Write-Output ("FG hwnd=0x{0:X} pid={1} proc={2} title={3}" -f $h.ToInt64(), $pid2, $(if ($proc) { $proc.ProcessName } else { "<gone>" }), $sb.ToString())
