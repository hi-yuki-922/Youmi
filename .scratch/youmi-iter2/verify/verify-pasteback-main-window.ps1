# ticket 01 on-machine verification driver: pasteback into the MAIN window.
# Precondition: youmi dev app running with CDP instrumentation (port 9223);
# gui-test-screenshots/cdp.mjs available. Keep this file ASCII-only: PowerShell
# 5.1 reads BOM-less scripts as ANSI and non-ASCII comments corrupt parsing.
# 1. Seed clipboard history: entry first, original last (original newest = row 1).
# 2. Point the main webview at /json and focus its input textarea (DOM focus).
# 3. Make the main window the OS foreground, Alt+V to summon the overlay,
#    {DOWN}{DOWN}{ENTER} selects row 2 (entry) and pastebacks.
# 4. Assert: the JSON input textarea received the entry text; user clipboard
#    restored to original.
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path "$PSScriptRoot/../../.."
$cdp = Join-Path $repoRoot "gui-test-screenshots\cdp.mjs"

Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Fg2 {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtra);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    public static string Title() {
        var sb = new StringBuilder(256);
        GetWindowText(GetForegroundWindow(), sb, 256);
        return sb.ToString();
    }
}
"@

$stamp = Get-Random -Maximum 9999
$entry = "youmi-iter2-pb-entry-$stamp"
$original = "youmi-iter2-pb-original-$stamp"

function Cdp([string]$urlMatch, [string]$expr) {
    $out = node $cdp $urlMatch $expr 2>&1
    if ($LASTEXITCODE -ne 0) { throw "cdp failed: $out" }
    return ($out -join "`n")
}

function Find-MainTarget {
    # Main window route is unknown; probe candidate routes (overlay targets
    # share the port, main routes are distinct), return the matching selector.
    foreach ($route in @("/json", "/diff", "/encoding", "/password-book", "/settings", "/")) {
        try {
            node $cdp "localhost:1420$route" "1" 2>$null | Out-Null
            if ($LASTEXITCODE -eq 0) { return "localhost:1420$route" }
        } catch {
            continue # no target on this route, try next candidate
        }
    }
    return $null
}

Write-Output "[1] Seed clipboard history (entry older, original newest)"
Set-Clipboard -Value $entry
Start-Sleep -Milliseconds 700
Set-Clipboard -Value $original
Start-Sleep -Milliseconds 700

Write-Output "[2] Main webview -> /json, focus input textarea"
$mainMatch = Find-MainTarget
if (-not $mainMatch) { throw "main window CDP target not found" }
Cdp $mainMatch "location.href = '/json'; 'nav'" | Out-Null
Start-Sleep -Seconds 2
$focused = Cdp "localhost:1420/json" "(() => {
  const box = document.querySelector('textarea');
  if (!box) return { error: 'no textarea' };
  box.focus();
  box.value = '';
  return { focused: document.activeElement === box };
})()"
if ($focused -notmatch '"focused": ?true') { throw "textarea not focused: $focused" }
Write-Output "    Input textarea focused"

Write-Output "[3] Bring Youmi main window to foreground"
$youmi = Get-Process youmi | Select-Object -First 1
if (-not $youmi -or $youmi.MainWindowHandle -eq 0) { throw "youmi main window not found" }
$hwnd = $youmi.MainWindowHandle
# Alt-tap releases the Windows foreground lock so SetForegroundWindow is granted
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) {
    if ([Fg2]::Title() -eq "Youmi") { break }
    [Fg2]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)  # Alt down
    [Fg2]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)  # Alt up
    [void][Fg2]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 300
}
if ([Fg2]::Title() -ne "Youmi") { throw "Youmi main window not foreground, foreground is: $([Fg2]::Title())" }
Write-Output "    Youmi main window is foreground"

Write-Output "[4] Alt+V summon overlay, {DOWN}{DOWN}{ENTER} pasteback row 2 (entry)"
[System.Windows.Forms.SendKeys]::SendWait("%v")
Start-Sleep -Milliseconds 1200
[System.Windows.Forms.SendKeys]::SendWait("{DOWN}")
Start-Sleep -Milliseconds 300
[System.Windows.Forms.SendKeys]::SendWait("{DOWN}")
Start-Sleep -Milliseconds 300
[System.Windows.Forms.SendKeys]::SendWait("{ENTER}")

Write-Output "[5] Wait for pasteback into JSON input (poll 8s)"
$pasted = $false
$boxValue = ""
$deadline = (Get-Date).AddSeconds(8)
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 400
    $boxValue = Cdp "localhost:1420/json" "document.querySelector('textarea')?.value ?? ''"
    if ($boxValue -match [regex]::Escape($entry)) { $pasted = $true; break }
}
if ($pasted) {
    Write-Output "PASS entry pasted into main window JSON input: [$boxValue]"
} else {
    Write-Output "FAIL JSON input did not receive entry, actual: [$boxValue]"
}

Write-Output "[6] Wait for user clipboard restore (poll 5s)"
$restored = $false
$clipNow = ""
$deadline = (Get-Date).AddSeconds(5)
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 400
    $clipNow = Get-Clipboard -Raw
    if ($clipNow -eq $original) { $restored = $true; break }
}
if ($restored) {
    Write-Output "PASS user clipboard restored: [$clipNow]"
} else {
    Write-Output "FAIL clipboard not restored, actual: [$clipNow] (expected [$original])"
}

if (-not ($pasted -and $restored)) { exit 1 }
Write-Output "ALL PASS"
