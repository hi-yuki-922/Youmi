# Plant a fake DeepSeek API key into the Windows credential store exactly the
# way keyring (windows-native) would store it, so the dev app can exercise the
# full request path (DeepSeek will reject the key with 401).
$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class Cred {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct CREDENTIAL {
        public int Flags;
        public int Type;
        public string TargetName;
        public string Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public int CredentialBlobSize;
        public IntPtr CredentialBlob;
        public int Persist;
        public int AttributeCount;
        public IntPtr Attributes;
        public string TargetAlias;
        public string UserName;
    }

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool CredWrite(ref CREDENTIAL credential, int flags);

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool CredDelete(string targetName, int type, int flags);

    public const int CRED_TYPE_GENERIC = 1;
    public const int CRED_PERSIST_LOCAL_MACHINE = 2;
}
"@

function Set-YoumiKey([string]$Value) {
    $bytes = [System.Text.Encoding]::Unicode.GetBytes($Value)
    $ptr = [System.Runtime.InteropServices.Marshal]::AllocHGlobal($bytes.Length)
    [System.Runtime.InteropServices.Marshal]::Copy($bytes, 0, $ptr, $bytes.Length)
    $cred = New-Object Cred+CREDENTIAL
    $cred.Type = [Cred]::CRED_TYPE_GENERIC
    # keyring v3 windows-native naming: target = "{user}.{service}", username = service
    $cred.TargetName = "deepseek-api-key.youmi"
    $cred.UserName = "youmi"
    $cred.CredentialBlob = $ptr
    $cred.CredentialBlobSize = $bytes.Length
    $cred.Persist = [Cred]::CRED_PERSIST_LOCAL_MACHINE
    $ok = [Cred]::CredWrite([ref]$cred, 0)
    [System.Runtime.InteropServices.Marshal]::FreeHGlobal($ptr)
    if (-not $ok) { throw "CredWrite failed: $([System.Runtime.InteropServices.Marshal]::GetLastWin32Error())" }
}

function Remove-YoumiKey {
    [void][Cred]::CredDelete("deepseek-api-key.youmi", [Cred]::CRED_TYPE_GENERIC, 0)
}

switch ($args[0]) {
    "set" { Set-YoumiKey $args[1]; Write-Output "key planted" }
    "remove" { Remove-YoumiKey; Write-Output "key removed" }
    default { Write-Output "usage: fake-key.ps1 set <value> | remove" }
}
