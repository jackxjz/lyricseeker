Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32 {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int W, int H, bool repaint);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@

# 用脚本自身位置推导项目根，避免非 ASCII 路径写死在文件里被编码破坏。
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root "src-tauri\target\release\lyricseeker.exe"
Write-Output "exe exists: $(Test-Path $exe)"

$proc = Start-Process -FilePath $exe -PassThru
Write-Output "started pid=$($proc.Id)"

$h = [IntPtr]::Zero
for ($i = 0; $i -lt 40; $i++) {
    Start-Sleep -Milliseconds 500
    $proc.Refresh()
    if ($proc.HasExited) { Write-Output "EXITED code=$($proc.ExitCode)"; exit 1 }
    $h = $proc.MainWindowHandle
    if ($h -ne [IntPtr]::Zero) { break }
}
Write-Output "hwnd=$h title=[$($proc.MainWindowTitle)]"
if ($h -eq [IntPtr]::Zero) { exit 2 }

[W32]::ShowWindow($h, 9) | Out-Null
[W32]::MoveWindow($h, 60, 40, 1180, 760, $true) | Out-Null
[W32]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 1500

$mode = $args[0]
if ($mode -ne "idle") {
    [System.Windows.Forms.SendKeys]::SendWait("晴天")
    Start-Sleep -Milliseconds 600
    [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
    Start-Sleep -Seconds 5

    if ($mode -eq "lyrics") {
        [System.Windows.Forms.SendKeys]::SendWait("{TAB}")
        Start-Sleep -Milliseconds 500
        [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
        Start-Sleep -Seconds 6
    }
}

$rect = New-Object W32+RECT
$ok = [W32]::GetWindowRect($h, [ref]$rect)
$w = [int]($rect.Right - $rect.Left)
$ht = [int]($rect.Bottom - $rect.Top)
Write-Output "getrect=$ok size=${w}x${ht}"
if ($w -le 10 -or $ht -le 10) { Write-Output "BAD RECT"; exit 3 }

$bmp = New-Object System.Drawing.Bitmap $w, $ht
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen([int]$rect.Left, [int]$rect.Top, 0, 0, $bmp.Size)
$dir = Join-Path $root "docs"
New-Item -ItemType Directory -Force $dir | Out-Null
$out = Join-Path $dir "shot-$mode.png"
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Output "saved=$out bytes=$((Get-Item $out).Length)"
Write-Output "pid_keep=$($proc.Id)"
