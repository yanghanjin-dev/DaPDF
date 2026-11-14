$ErrorActionPreference = "Stop"

$VersionNum = "0.0.2"
$Tag = "v$VersionNum"
$Repo = "yanghanjin-dev/DaPDF"
$DistDir = "dist"
$ZipName = "DaPDF-$Tag-win-x64.zip"

Write-Host "[1/5] Setting version to $VersionNum..." -ForegroundColor Cyan
Set-Content -Path "VERSION" -Value $VersionNum -Encoding Ascii -NoNewline

Write-Host "[2/5] Building Rust Release binary (cargo build --release)..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Cargo build failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "[3/5] Creating distribution folder ($DistDir)..." -ForegroundColor Cyan
if (Test-Path $DistDir) { Remove-Item -Recurse -Force $DistDir }
New-Item -ItemType Directory -Path $DistDir | Out-Null

Copy-Item "target\release\dapdf.exe" -Destination "$DistDir\dapdf.exe"
Copy-Item "vendor\bin\pdfium.dll" -Destination "$DistDir\pdfium.dll"
if (Test-Path "README.md") { Copy-Item "README.md" -Destination "$DistDir\README.md" }
if (Test-Path "vendor\LICENSE") { Copy-Item "vendor\LICENSE" -Destination "$DistDir\LICENSE-PDFium.txt" }

Write-Host "[4/5] Compressing archive ($ZipName)..." -ForegroundColor Cyan
if (Test-Path $ZipName) { Remove-Item -Force $ZipName }
Compress-Archive -Path "$DistDir\*" -DestinationPath $ZipName

Write-Host "[5/5] Opening GitHub Releases page..." -ForegroundColor Green
Write-Host "SUCCESS! Archive created: $ZipName" -ForegroundColor Green
Start-Process "https://github.com/$Repo/releases/new?tag=$Tag&title=DaPDF+$Tag"
