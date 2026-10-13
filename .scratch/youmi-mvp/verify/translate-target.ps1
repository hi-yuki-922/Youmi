# Scriptable translate target: WinForms form whose multiline textbox holds
# preset text, so the driver can select-all and trigger Alt+T on it.
param([string]$OutFile = "$env:TEMP\youmi-translate-out.txt")

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$form = New-Object System.Windows.Forms.Form
$form.Text = "youmi-translate-target"
$form.Size = New-Object System.Drawing.Size(520, 300)

$box = New-Object System.Windows.Forms.TextBox
$box.Multiline = $true
$box.Dock = "Fill"
$box.Text = "The quick brown fox jumps over the lazy dog."
$form.Controls.Add($box)

$outPath = $OutFile
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 300
$timer.Add_Tick({ [IO.File]::WriteAllText($outPath, $box.Text) })
$timer.Start()

$form.Add_Shown({ $box.Focus() })
[void][System.Windows.Forms.Application]::Run($form)
