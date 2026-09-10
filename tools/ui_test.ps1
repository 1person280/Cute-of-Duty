# UI visual test helper for cod1-demo (ASCII only; PS 5.1 BOM-less safe).
# Actions:
#   shot  - capture game client area to OutFile
#   click - move cursor to client-space UiX,UiY (logical px) and left-click
#   key   - tap virtual key Vk
param(
    [Parameter(Mandatory=$true)][string]$Action,
    [string]$OutFile,
    [float]$UiX = 0,
    [float]$UiY = 0,
    [int]$Vk = 0
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Native {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public struct POINT { public int X, Y; }
}
"@

[Native]::SetProcessDPIAware() | Out-Null
$proc = Get-Process -Name 'cod1-demo' -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { throw 'game process/window not found' }
$h = [IntPtr]$proc.MainWindowHandle
[Native]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 150

$rect = New-Object Native+RECT
[Native]::GetClientRect($h, [ref]$rect) | Out-Null
$origin = New-Object Native+POINT
$origin.X = 0; $origin.Y = 0
[Native]::ClientToScreen($h, [ref]$origin) | Out-Null
$scale = [Native]::GetDpiForWindow($h) / 96.0
$clientW = $rect.Right - $rect.Left
$clientH = $rect.Bottom - $rect.Top

switch ($Action) {
    'shot' {
        $bmp = New-Object System.Drawing.Bitmap($clientW, $clientH)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($origin.X, $origin.Y, 0, 0, (New-Object System.Drawing.Size($clientW, $clientH)))
        $bmp.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose(); $bmp.Dispose()
        Write-Host ("shot saved: " + $OutFile + " client=" + $clientW + "x" + $clientH + " scale=" + $scale)
    }
    'click' {
        $px = [int]($origin.X + $UiX * $scale)
        $py = [int]($origin.Y + $UiY * $scale)
        [Native]::SetCursorPos($px, $py) | Out-Null
        Start-Sleep -Milliseconds 120
        [Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)  # LEFTDOWN
        Start-Sleep -Milliseconds 70
        [Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)  # LEFTUP
        Start-Sleep -Milliseconds 200
        Write-Host ("clicked ui(" + $UiX + "," + $UiY + ") -> screen(" + $px + "," + $py + ")")
    }
    'rawclick' {
        # click at current cursor position without moving (in-game fire: avoids
        # huge mouse deltas from SetCursorPos while cursor is locked)
        [Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 70
        [Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 200
        Write-Host "raw clicked at current position"
    }
    'key' {
        [Native]::keybd_event([byte]$Vk, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 60
        [Native]::keybd_event([byte]$Vk, 0, 2, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 150
        Write-Host ("tapped vk " + $Vk)
    }
    default { throw "unknown action: $Action" }
}
