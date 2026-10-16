# ticket 03 on-machine verification driver: lingering modifier keys are cleared
# right after an overlay is summoned via global hotkey.
# Precondition: youmi dev app running with CDP instrumentation (port 9223);
# gui-test-screenshots/cdp.mjs available. Keep this file ASCII-only.
# Scenario A (clipboard overlay, Alt+V): hold Alt down (user still gripping the
#   modifier when the hotkey fires), press V, wait for the overlay. Assert:
#   GetAsyncKeyState(Alt) reports UP (Rust cleanup released it), then a plain
#   Down moves list selection (search box -> first row) instead of being
#   swallowed by the stuck Alt / system menu.
# Scenario B (command palette, super+alt+space): same with Win+Alt held down;
#   assert both Win and Alt report UP after summon and Down advances
#   aria-activedescendant from palette-item-0 to palette-item-1.
# Before each summon the Youmi main window is made foreground: hiding a focused
# overlay can leave the desktop without any foreground window, and a summon
# from that state cannot take focus (SetForegroundWindow denied) -- a real user
# always summons FROM some app, so re-establish that precondition.
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path "$PSScriptRoot/../../.."
$cdp = Join-Path $repoRoot "gui-test-screenshots\cdp.mjs"

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Keys3 {
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtra);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vKey);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    public static bool IsDown(int vk) { return (GetAsyncKeyState(vk) & 0x8000) != 0; }
    public static IntPtr ForegroundHwnd() { return GetForegroundWindow(); }
    public static void Tap(int vk) { keybd_event((byte)vk, 0, 0, UIntPtr.Zero); keybd_event((byte)vk, 0, 2, UIntPtr.Zero); }
    public static void Down(int vk) { keybd_event((byte)vk, 0, 0, UIntPtr.Zero); }
    public static void Up(int vk) { keybd_event((byte)vk, 0, 2, UIntPtr.Zero); }
}
"@

$VK_MENU = 0x12; $VK_LWIN = 0x5B; $VK_V = 0x56; $VK_SPACE = 0x20; $VK_DOWN = 0x28; $VK_ESCAPE = 0x1B
$failures = @()

function Cdp([string]$urlMatch, [string]$expr) {
    $out = node $cdp $urlMatch $expr 2>&1
    if ($LASTEXITCODE -ne 0) { throw "cdp failed: $out" }
    return ($out -join "`n")
}

function Set-ForegroundToYoumi {
    $youmiHwnd = (Get-Process youmi | Select-Object -First 1).MainWindowHandle
    if (-not $youmiHwnd) { throw "youmi main window not found" }
    $deadline = (Get-Date).AddSeconds(10)
    while ((Get-Date) -lt $deadline) {
        [Keys3]::Tap($VK_MENU)  # Alt-tap releases the Windows foreground lock
        [void][Keys3]::SetForegroundWindow($youmiHwnd)
        Start-Sleep -Milliseconds 400
        if ([Keys3]::ForegroundHwnd() -eq $youmiHwnd) { return }
    }
    throw "could not establish Youmi main window as foreground (desktop session state)"
}

Write-Output "[A1] Hold Alt down, tap V (global hotkey Alt+V)"
Set-ForegroundToYoumi
[Keys3]::Down($VK_MENU)
Start-Sleep -Milliseconds 80
[Keys3]::Down($VK_V)
Start-Sleep -Milliseconds 60
[Keys3]::Up($VK_V)
Start-Sleep -Milliseconds 1500

if ([Keys3]::IsDown($VK_MENU)) {
    $failures += "Alt still reported DOWN after overlay summon (cleanup did not fire)"
    [Keys3]::Up($VK_MENU)
} else {
    Write-Output "PASS Alt async key state is UP after overlay summon"
}

Write-Output "[A2] Plain Down should move clipboard list selection (search box -> row)"
[Keys3]::Tap($VK_DOWN)
Start-Sleep -Milliseconds 400
$active = Cdp "localhost:1420/overlay/clipboard" "document.activeElement.tagName + '|' + (document.activeElement.getAttribute('aria-label') ?? '')"
if ($active -match '^"LI\|') {
    Write-Output "PASS Down focused first list row: $active"
} else {
    $failures += "Down did not reach clipboard list, activeElement: $active"
}

Write-Output "[A3] Hide clipboard overlay (Alt+V toggle), then restore a foreground"
[Keys3]::Down($VK_MENU)
Start-Sleep -Milliseconds 80
[Keys3]::Tap($VK_V)
[Keys3]::Up($VK_MENU)
Start-Sleep -Milliseconds 800

Write-Output "[B1] Hold Win+Alt down, tap Space (global hotkey super+alt+space)"
Set-ForegroundToYoumi
[Keys3]::Down($VK_LWIN)
Start-Sleep -Milliseconds 80
[Keys3]::Down($VK_MENU)
Start-Sleep -Milliseconds 80
[Keys3]::Tap($VK_SPACE)
Start-Sleep -Milliseconds 1500

if ([Keys3]::IsDown($VK_LWIN) -or [Keys3]::IsDown($VK_MENU)) {
    $failures += ("Win/Alt still reported DOWN after palette summon (win={0} alt={1})" -f [Keys3]::IsDown($VK_LWIN), [Keys3]::IsDown($VK_MENU))
    [Keys3]::Up($VK_MENU)
    [Keys3]::Up($VK_LWIN)
} else {
    Write-Output "PASS Win and Alt async key states are UP after palette summon"
}

Write-Output "[B2] Plain Down should advance palette active item"
$before = Cdp "localhost:1420/overlay/palette" "document.querySelector('input[role=combobox]')?.getAttribute('aria-activedescendant') ?? 'none'"
[Keys3]::Tap($VK_DOWN)
Start-Sleep -Milliseconds 400
$after = Cdp "localhost:1420/overlay/palette" "document.querySelector('input[role=combobox]')?.getAttribute('aria-activedescendant') ?? 'none'"
if ($before -match "palette-item-0" -and $after -match "palette-item-1") {
    Write-Output "PASS palette active item advanced: $before -> $after"
} else {
    $failures += "palette selection did not advance, before: $before after: $after"
}

Write-Output "[B3] Hide palette (Esc)"
[Keys3]::Tap($VK_ESCAPE)
Start-Sleep -Milliseconds 500

# Release everything just in case a failure path left keys held
[Keys3]::Up($VK_MENU); [Keys3]::Up($VK_LWIN)

if ($failures.Count -gt 0) { $failures | ForEach-Object { Write-Output "FAIL $_" }; exit 1 }
Write-Output "ALL PASS"
