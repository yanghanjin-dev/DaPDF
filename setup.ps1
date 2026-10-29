$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
New-Item -ItemType Directory -Force vendor | Out-Null
# Public PDFium binary distribution, without V8 / JavaScript.
Invoke-WebRequest 'https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz' -OutFile vendor/pdfium.tgz
tar -xzf vendor/pdfium.tgz -C vendor
if ($LASTEXITCODE -ne 0) { throw 'PDFium 압축 해제 실패' }
Write-Host 'PDFium 설치 완료. .\build.ps1을 실행하세요.'
