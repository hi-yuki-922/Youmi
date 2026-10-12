# Scriptable pasteback target: WinForms form with a multiline textbox, dumping
# textbox content to a file every 300ms so the driver can assert the paste.
param([string]$OutFile = "$env:TEMP\youmi-pasteback-out.txt")

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$form = New-Object System.Windows.Forms.Form
$form.Text = "youmi-pasteback-target"
$form.Size = New-Object System.Drawing.Size(500, 300)

$box = New-Object System.Windows.Forms.TextBox
$box.Multiline = $true
$box.Dock = "Fill"
$form.Controls.Add($box)

$outPath = $OutFile
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 300
$timer.Add_Tick({ [IO.File]::WriteAllText($outPath, $box.Text) })
$timer.Start()

$form.Add_Shown({ $box.Focus() })
[void][System.Windows.Forms.Application]::Run($form)
