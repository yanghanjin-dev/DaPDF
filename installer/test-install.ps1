param([Parameter(Mandatory = $true)][string]$StageDir)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Workspace = Split-Path $PSScriptRoot -Parent
$TestRoot = Join-Path $Workspace ('target\installer-test\' + [guid]::NewGuid().ToString('N'))
$InstallDir = Join-Path $TestRoot 'installed'
$TestUninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{0A3AA95A-CCF1-4475-97F5-7C55F6A4DAE1}_is1'
$ProgKey = 'HKCU:\Software\Classes\DaPDF.InstallerTest.PDF'
$ApplicationKey = 'HKCU:\Software\Classes\Applications\DaPDF-InstallerTest.exe'
$Group = Join-Path ([Environment]::GetFolderPath('Programs')) 'DaPDF Installer Test'
$DesktopLink = Join-Path ([Environment]::GetFolderPath('Desktop')) 'DaPDF Installer Test.lnk'
foreach ($path in @($TestUninstallKey, $ProgKey, $ApplicationKey, $Group, $DesktopLink)) {
    if (Test-Path -LiteralPath $path) { throw "Existing test installation must be removed before testing: $path" }
}
# Test identity and per-user scope isolate the smoke test from real administrator installs.
# Production uses PrivilegesRequired=admin; Inno's loader manifest is deliberately
# asInvoker and the setup engine requests UAC elevation after unpacking.
& (Join-Path $Workspace 'build-installer.ps1') -StageDir $StageDir -OutputDir $TestRoot -InstallerTest
$VersionNum = (Get-Content -LiteralPath (Join-Path $Workspace 'VERSION') -Raw).Trim()
$Installer = Join-Path $TestRoot "DaPDF-v$VersionNum-win-x64-setup.exe"
function Invoke-Checked([string]$Executable, [string[]]$Arguments) {
    $process = Start-Process -FilePath $Executable -ArgumentList $Arguments -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Process failed ($($process.ExitCode)): $Executable" }
}
$InstallArguments = @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-',('/DIR="' + $InstallDir + '"'),'/TASKS=desktopicon,pdfopenwith')
Invoke-Checked $Installer ($InstallArguments + ('/LOG="' + (Join-Path $TestRoot 'install.log') + '"'))
foreach ($path in @((Join-Path $InstallDir 'DaPDF.exe'), (Join-Path $InstallDir 'pdfium.dll'), (Join-Path $InstallDir 'unins000.exe'), $TestUninstallKey, $ProgKey, $ApplicationKey, $Group, $DesktopLink)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing installed artifact: $path" }
}
if ((Get-FileHash -LiteralPath (Join-Path $InstallDir 'DaPDF.exe')).Hash -ne (Get-FileHash -LiteralPath (Join-Path $StageDir 'DaPDF.exe')).Hash) { throw 'Installed executable hash mismatch.' }
$OpenCommand = (Get-Item -LiteralPath ($ProgKey + '\shell\open\command')).GetValue('')
if ($OpenCommand -ne ('"' + $InstallDir + '\DaPDF.exe" "%1"')) { throw 'Incorrect Open With command quoting.' }
Write-Host 'PASS: files, uninstall registration, shortcuts and PDF Open With registration.'
$Sentinel = Join-Path $InstallDir 'user-document.txt'
Set-Content -LiteralPath $Sentinel -Value 'Preserve this user-created document.'
Invoke-Checked $Installer ($InstallArguments + ('/LOG="' + (Join-Path $TestRoot 'reinstall.log') + '"'))
if (-not (Test-Path -LiteralPath $Sentinel)) { throw 'Reinstall removed the user document.' }
Write-Host 'PASS: reinstall preserves user documents.'
Invoke-Checked (Join-Path $InstallDir 'unins000.exe') @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="' + (Join-Path $TestRoot 'uninstall.log') + '"'))
for ($attempt = 0; $attempt -lt 30 -and (Test-Path -LiteralPath (Join-Path $InstallDir 'unins000.exe')); $attempt++) { Start-Sleep -Milliseconds 200 }
foreach ($path in @((Join-Path $InstallDir 'DaPDF.exe'), (Join-Path $InstallDir 'pdfium.dll'), (Join-Path $InstallDir 'unins000.exe'), $TestUninstallKey, $ProgKey, $ApplicationKey, $Group, $DesktopLink)) {
    if (Test-Path -LiteralPath $path) { throw "Uninstall left an installed artifact: $path" }
}
$PdfOpenWithKey = Get-Item -LiteralPath 'HKCU:\Software\Classes\.pdf\OpenWithProgids'
if ($PdfOpenWithKey.GetValueNames() -contains 'DaPDF.InstallerTest.PDF') { throw 'Uninstall left the PDF Open With value.' }
if (-not (Test-Path -LiteralPath $Sentinel)) { throw 'Uninstall removed a user-created document.' }
Write-Host 'PASS: uninstall removes installed files, registration and shortcuts; user document remains.'
Write-Host "Logs and preserved test document: $TestRoot"
