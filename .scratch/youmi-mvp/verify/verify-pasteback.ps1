# ticket 07 pasteback on-machine verification driver.
# Precondition: youmi dev app is already running.
# 1. Seed clipboard history: copy original, then entry (entry newest = first row).
# 2. Start paste target form, make sure it is foreground.
# 3. Alt+V to summon overlay, {DOWN}{ENTER} to pasteback first row.
# 4. Assert: target textbox received entry text; clipboard restored to original.
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Windows.Forms
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

$outFile = "$env:TEMP\youmi-pasteback-out.txt"
Remove-Item $outFile -ErrorAction SilentlyContinue

$entry = "youmi-pb-entry-abc123"
$original = "youmi-pb-original-xyz789"

# entry first (older history item), original last: original is the clipboard
# state at summon time and must be restored after pasting the older entry.
Write-Output "[1] Seed clipboard history (entry older, original newest)"
Set-Clipboard -Value $entry
Start-Sleep -Milliseconds 700
Set-Clipboard -Value $original
Start-Sleep -Milliseconds 700

Write-Output "[2] Start paste target form"
$target = Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "$PSScriptRoot\paste-target.ps1", "-OutFile", $outFile -PassThru
Start-Sleep -Seconds 3

$shell = New-Object -ComObject WScript.Shell
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) {
    if ([Fg]::Title() -like "*youmi-pasteback-target*") { break }
    [void]$shell.AppActivate("youmi-pasteback-target")
    Start-Sleep -Milliseconds 300
}
if ([Fg]::Title() -notlike "*youmi-pasteback-target*") {
    throw "Target form not in foreground, foreground is: $([Fg]::Title())"
}
Write-Output "    Target form is foreground"

Write-Output "[3] Alt+V to summon overlay"
[System.Windows.Forms.SendKeys]::SendWait("%v")
Start-Sleep -Milliseconds 1200

Write-Output "[4] {DOWN}{DOWN}{ENTER} select second row (older entry) and pasteback"
[System.Windows.Forms.SendKeys]::SendWait("{DOWN}")
Start-Sleep -Milliseconds 300
[System.Windows.Forms.SendKeys]::SendWait("{DOWN}")
Start-Sleep -Milliseconds 300
[System.Windows.Forms.SendKeys]::SendWait("{ENTER}")

Write-Output "[5] Wait for pasteback + restore (3s)"
Start-Sleep -Seconds 3

$failures = @()
$pasted = if (Test-Path $outFile) { Get-Content $outFile -Raw } else { "" }
if ($pasted -notmatch [regex]::Escape($entry)) {
    $failures += "Target textbox did not receive entry, actual: [$($pasted.Trim())]"
} else {
    Write-Output "PASS paste arrived at target window: [$($pasted.Trim())]"
}
$clip = Get-Clipboard -Raw
if ($clip -ne $original) {
    $failures += "Clipboard not restored to original, actual: [$clip]"
} else {
    Write-Output "PASS clipboard restored to original: [$clip]"
}

Write-Output "[6] Cleanup"
Stop-Process -Id $target.Id -Force -ErrorAction SilentlyContinue
if ($failures.Count -gt 0) { $failures | ForEach-Object { Write-Output "FAIL $_" }; exit 1 }
Write-Output "ALL PASS"
