param(
    [string]$Exe = "C:\lsrun\lyricseeker.exe",
    [string]$Out = "C:\lsrun\shot.png",
    [int]$WaitSec = 9
)

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;

public class WinCap {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder s, int n);

    public struct RECT { public int Left, Top, Right, Bottom; }

    public static string Title(IntPtr h) {
        StringBuilder sb = new StringBuilder(512);
        GetWindowTextW(h, sb, 512);
        return sb.ToString();
    }

    public static bool Capture(IntPtr h, string path) {
        ShowWindow(h, 9);
        SetForegroundWindow(h);
        System.Threading.Thread.Sleep(600);
        RECT r;
        if (!GetWindowRect(h, out r)) return false;
        int w = r.Right - r.Left, ht = r.Bottom - r.Top;
        if (w <= 0 || ht <= 0) return false;
        using (Bitmap bmp = new Bitmap(w, ht, PixelFormat.Format32bppArgb)) {
            using (Graphics g = Graphics.FromImage(bmp)) {
                IntPtr hdc = g.GetHdc();
                PrintWindow(h, hdc, 0);
                g.ReleaseHdc(hdc);
            }
            bmp.Save(path, ImageFormat.Png);
        }
        return true;
    }
}
"@

[WinCap]::SetProcessDPIAware() | Out-Null

Get-Process -Name lyricseeker -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 400

if (Test-Path $Exe) { Unblock-File -Path $Exe -ErrorAction SilentlyContinue }

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Exe
$psi.UseShellExecute = $false
$psi.WorkingDirectory = Split-Path $Exe
$proc = [System.Diagnostics.Process]::Start($psi)
Write-Output "started pid=$($proc.Id)"
Start-Sleep -Seconds $WaitSec

if ($proc.HasExited) {
    Write-Output "EXITED code=$($proc.ExitCode)"
    exit 1
}

$h = $proc.MainWindowHandle
if ($h -eq [IntPtr]::Zero) {
    for ($i = 0; $i -lt 20; $i++) {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $h = $proc.MainWindowHandle; break }
    }
}

Write-Output "hwnd=$h title='$([WinCap]::Title($h))'"
if ($h -eq [IntPtr]::Zero) { Write-Output "no main window"; exit 1 }

if (-not (Test-Path (Split-Path $Out))) { New-Item -ItemType Directory -Force (Split-Path $Out) | Out-Null }
if ([WinCap]::Capture($h, $Out)) {
    $fi = Get-Item $Out
    Write-Output "saved $($fi.FullName) $($fi.Length) bytes"
} else {
    Write-Output "capture failed"
    exit 1
}
