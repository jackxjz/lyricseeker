param(
    [string]$Out = "E:\Code 编程\DeepSeek Harness\LyricSeeker\docs\screenshot-kugou.png"
)

Add-Type -AssemblyName System.Drawing

$src = @'
using System;
using System.Drawing;
using System.Runtime.InteropServices;

public class Cap2 {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);

    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    public static bool Grab(IntPtr h, string path) {
        if (IsIconic(h)) { ShowWindow(h, 9); }
        ShowWindow(h, 9);
        SetForegroundWindow(h);
        System.Threading.Thread.Sleep(900);
        RECT r;
        if (!GetWindowRect(h, out r)) { return false; }
        int w = r.Right - r.Left;
        int ht = r.Bottom - r.Top;
        if (w <= 0 || ht <= 0) { return false; }
        using (Bitmap bmp = new Bitmap(w, ht)) {
            using (Graphics g = Graphics.FromImage(bmp)) {
                g.CopyFromScreen(r.Left, r.Top, 0, 0, new Size(w, ht));
            }
            bmp.Save(path, System.Drawing.Imaging.ImageFormat.Png);
        }
        return true;
    }
}
'@

Add-Type -TypeDefinition $src -ReferencedAssemblies System.Drawing
[Cap2]::SetProcessDPIAware() | Out-Null

$p = Get-Process lyricseeker -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output "not running"; exit 1 }
$p.Refresh()
$h = $p.MainWindowHandle
Write-Output "pid=$($p.Id) hwnd=$h title='$($p.MainWindowTitle)'"
if ($h -eq [IntPtr]::Zero) { Write-Output "no window"; exit 1 }

$dir = Split-Path $Out
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force $dir | Out-Null }

if ([Cap2]::Grab($h, $Out)) {
    $f = Get-Item $Out
    Write-Output "saved $($f.FullName) $($f.Length) bytes"
} else {
    Write-Output "grab failed"
    exit 1
}
