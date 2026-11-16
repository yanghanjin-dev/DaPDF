param([switch]$SkipTests, [switch]$SkipInstaller)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location $PSScriptRoot
try {
    $VersionNum = (Get-Content -LiteralPath 'VERSION' -Raw).Trim()
    $CargoText = Get-Content -LiteralPath 'Cargo.toml' -Raw
    if ($VersionNum -notmatch '^\d+\.\d+\.\d+$' -or $CargoText -notmatch ('(?m)^version\s*=\s*"' + [regex]::Escape($VersionNum) + '"')) {
        throw 'VERSION and Cargo.toml must contain the same semantic version.'
    }
    # PDFium must also be discoverable when Cargo places test executables in deps/.
    $env:PATH = "$PSScriptRoot\vendor\bin;$env:PATH"
    if (-not $SkipTests) {
        cargo test --offline
        if ($LASTEXITCODE -ne 0) { throw 'Regression tests failed.' }
    }
    cargo build --release --offline --target-dir target/distribution
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $ReleaseDir = Join-Path $PSScriptRoot 'releases'
    $Stage = Join-Path $PSScriptRoot ('dist\v' + $VersionNum + '-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $Stage -Force | Out-Null
    New-Item -ItemType Directory -Path $ReleaseDir -Force | Out-Null
    Copy-Item -LiteralPath 'target\distribution\release\dapdf.exe' -Destination (Join-Path $Stage 'DaPDF.exe')
    Copy-Item -LiteralPath 'vendor\bin\pdfium.dll' -Destination $Stage
    Copy-Item -LiteralPath 'README.md','VERSION' -Destination $Stage
    Copy-Item -LiteralPath "releases\notes-v$VersionNum.md" -Destination (Join-Path $Stage 'RELEASE-NOTES.md')
    Copy-Item -LiteralPath 'vendor\LICENSE' -Destination (Join-Path $Stage 'LICENSE-PDFium.txt')
    Copy-Item -LiteralPath 'vendor\licenses' -Destination (Join-Path $Stage 'licenses') -Recurse
    $ZipName = "DaPDF-v$VersionNum-win-x64.zip"
    $ZipPath = Join-Path $ReleaseDir $ZipName
    Compress-Archive -Path (Join-Path $Stage '*') -DestinationPath $ZipPath -Force
    $Hash = (Get-FileHash -LiteralPath $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $ReleaseDir "$ZipName.sha256") -Value "$Hash  $ZipName" -Encoding Ascii
    Write-Host "Package: $ZipPath"
    Write-Host "SHA256: $Hash"
    Write-Host "Staging: $Stage"
    if (-not $SkipInstaller) {
        & (Join-Path $PSScriptRoot 'build-installer.ps1') -StageDir $Stage
    }
} finally {
    Pop-Location
}
