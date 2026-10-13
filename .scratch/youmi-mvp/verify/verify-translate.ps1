# ticket 11 selection-translate on-machine verification driver.
# Precondition: youmi dev app is already running; a fake DeepSeek key is planted
# in the credential store (deepseek-api-key.youmi) so the request path runs and
# DeepSeek rejects it with 401 - the overlay must surface that error clearly.
# 1. Alt+T with no selectable text: overlay shows the nothing-selected message.
# 2. Seed user clipboard, start text form (foreign process), select all, Alt+T:
#    overlay appears near cursor with the 401 error message.
# 3. Assert user clipboard content is restored after the capture.
# 4. Alt+V: clipboard history must NOT contain the selected text (simulated copy
#    is filtered out of history).
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Fg {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    public static string Title() {
        var sb = new StringBuilder(256);
        GetWindowText(GetForegroundWindow(), sb, 256);
        return sb.ToString();
    }
}
"@

$shots = Join-Path $PSScriptRoot "..\..\..\gui-test-screenshots"

function Save-Shot([string]$Name) {
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $g.Dispose()
    $bmp.Save((Join-Path $shots $Name), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Output "    screenshot: $Name"
}

Write-Output "[1] Alt+T with nothing selected (driver console is foreground)"
[System.Windows.Forms.SendKeys]::SendWait("%t")
Start-Sleep -Milliseconds 2000
Save-Shot "ticket11-1-no-selection.png"

Write-Output "[2] Seed user clipboard"
$original = "youmi-translate-original-xyz"
Set-Clipboard -Value $original
Start-Sleep -Milliseconds 700

Write-Output "[3] Start text form (foreign process) and select all"
$outFile = "$env:TEMP\youmi-translate-out.txt"
Remove-Item $outFile -ErrorAction SilentlyContinue
$target = Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "$PSScriptRoot\translate-target.ps1", "-OutFile", $outFile -PassThru
Start-Sleep -Seconds 3

$shell = New-Object -ComObject WScript.Shell
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) {
    if ([Fg]::Title() -like "*youmi-translate-target*") { break }
    [void]$shell.AppActivate("youmi-translate-target")
    Start-Sleep -Milliseconds 300
}
if ([Fg]::Title() -notlike "*youmi-translate-target*") {
    throw "Target form not in foreground, foreground is: $([Fg]::Title())"
}
Write-Output "    Target form is foreground"
[System.Windows.Forms.SendKeys]::SendWait("^a")
Start-Sleep -Milliseconds 300

Write-Output "[4] Alt+T to translate the selection (fake key -> expect 401 error in overlay)"
[System.Windows.Forms.SendKeys]::SendWait("%t")
Start-Sleep -Milliseconds 3000
Save-Shot "ticket11-2-translate-overlay.png"

Write-Output "[5] Assert user clipboard restored"
$clip = Get-Clipboard
if ($clip -ne $original) { throw "clipboard not restored, got: [$clip]" }
Write-Output "    clipboard restored OK"

Write-Output "[6] Alt+V: history must not contain the selected text"
[System.Windows.Forms.SendKeys]::SendWait("%v")
Start-Sleep -Milliseconds 1500
Save-Shot "ticket11-3-clipboard-history.png"
[System.Windows.Forms.SendKeys]::SendWait("%v")
Start-Sleep -Milliseconds 600

Write-Output "[7] Cleanup target form"
Stop-Process -Id $target.Id -Force -ErrorAction SilentlyContinue

Write-Output "DONE"
