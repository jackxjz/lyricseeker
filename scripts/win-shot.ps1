# Bring a window to the front (topmost) and capture its rect including the native title bar.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/win-shot.ps1 -Out shot.png [-ProcessName lyricseeker]
# Keep this file ASCII-only: PowerShell 5.1 reads BOM-less UTF-8 as GBK.
param(
  [Parameter(Mandatory = $true)][string]$Out,
  [string]$ProcessName = "lyricseeker"
)

Add-Type -AssemblyName System.Drawing

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Win32Shot {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$HWND_TOPMOST = [IntPtr](-1)
$HWND_NOTOPMOST = [IntPtr](-2)
$SWP_NOMOVE = 0x0002
$SWP_NOSIZE = 0x0001

$proc = Get-Process -Name $ProcessName -ErrorAction Stop |
  Where-Object { $_.MainWindowHandle -ne 0 } |
  Select-Object -First 1
if (-not $proc) { throw "no window found for $ProcessName" }

$h = $proc.MainWindowHandle
[void][Win32Shot]::ShowWindow($h, 9)   # SW_RESTORE
# Foreground lock keeps SetForegroundWindow from stealing focus; topmost does not need it.
[void][Win32Shot]::SetWindowPos($h, $HWND_TOPMOST, 0, 0, 0, 0, $SWP_NOMOVE -bor $SWP_NOSIZE)
[void][Win32Shot]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 900

$rect = New-Object Win32Shot+RECT
[void][Win32Shot]::GetWindowRect($h, [ref]$rect)
$w = $rect.Right - $rect.Left
$hh = $rect.Bottom - $rect.Top

$bmp = New-Object System.Drawing.Bitmap($w, $hh)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$g.Dispose()
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

[void][Win32Shot]::SetWindowPos($h, $HWND_NOTOPMOST, 0, 0, 0, 0, $SWP_NOMOVE -bor $SWP_NOSIZE)

Write-Output "saved $Out ($w x $hh) title=$($proc.MainWindowTitle)"
