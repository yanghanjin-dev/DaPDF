param(
    [Parameter(Mandatory = $true)][string]$StageDir,
    [string]$CompilerPath,
    [string]$OutputDir,
    [switch]$InstallerTest
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$VersionNum = (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'VERSION') -Raw).Trim()
if ($VersionNum -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid VERSION.' }
$StageDir = (Resolve-Path -LiteralPath $StageDir).Path
foreach ($file in @('DaPDF.exe','pdfium.dll','README.md','VERSION','RELEASE-NOTES.md','LICENSE-PDFium.txt','licenses')) {
    if (-not (Test-Path -LiteralPath (Join-Path $StageDir $file))) { throw "Missing installer payload: $file" }
}
if ((Get-Content -LiteralPath (Join-Path $StageDir 'VERSION') -Raw).Trim() -ne $VersionNum) {
    throw 'Staged VERSION does not match the project.'
}
if (-not $CompilerPath) {
    $candidates = @((Join-Path $PSScriptRoot '.tools\inno\ISCC.exe'))
    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) { $candidates += $command.Source }
    foreach ($base in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if ($base) { $candidates += Join-Path $base 'Inno Setup 6\ISCC.exe'; $candidates += Join-Path $base 'Inno Setup 7\ISCC.exe' }
    }
    $CompilerPath = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $CompilerPath) { throw 'Inno Setup compiler not found. Install Inno Setup or pass -CompilerPath.' }
if (-not $OutputDir) { $OutputDir = Join-Path $PSScriptRoot 'releases' }
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
$arguments = @('/Qp', "/DAppVersion=$VersionNum", "/DSourceDir=$StageDir", "/DOutputDir=$OutputDir")
if ($InstallerTest) { $arguments += '/DInstallerTest' }
$arguments += Join-Path $PSScriptRoot 'installer\DaPDF.iss'
& $CompilerPath @arguments
if ($LASTEXITCODE -ne 0) { throw "Installer compilation failed: $LASTEXITCODE" }
$Installer = Join-Path $OutputDir "DaPDF-v$VersionNum-win-x64-setup.exe"
$Hash = (Get-FileHash -LiteralPath $Installer -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath "$Installer.sha256" -Value "$Hash  $([IO.Path]::GetFileName($Installer))" -Encoding Ascii
Write-Host "Installer: $Installer"
