param([switch]$Test)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
New-Item -ItemType Directory -Force dist | Out-Null
$compiler = Get-Command gcc -ErrorAction SilentlyContinue
if ($compiler) { $gcc = $compiler.Source }
elseif (Test-Path 'C:\msys64\ucrt64\bin\gcc.exe') { $gcc = 'C:\msys64\ucrt64\bin\gcc.exe' }
else { throw '64비트 MinGW-w64 GCC를 설치하고 PATH에 추가하세요.' }
if (!(Test-Path 'vendor\include\fpdfview.h')) { throw '먼저 .\setup.ps1을 실행하세요.' }
$env:PATH = (Split-Path $gcc) + ';' + $env:PATH
$libs = '-static-libgcc', '-lcomdlg32', '-lshell32', '-lgdi32', '-luser32'
& $gcc main.c vendor/bin/pdfium.dll -o dist/DaPDF.exe -std=c11 -O2 -Wall -Wextra -Werror -municode -mwindows @libs
if ($LASTEXITCODE -ne 0) { throw '컴파일 실패' }
Copy-Item vendor/bin/pdfium.dll dist/pdfium.dll -Force
Copy-Item vendor/LICENSE* dist -Force
# 대상 폴더가 이미 있으면 Copy-Item이 그 안에 licenses를 한 겹 더 만들므로 먼저 지운다.
if (Test-Path dist/licenses) { Remove-Item dist/licenses -Recurse -Force }
Copy-Item vendor/licenses dist -Recurse -Force
Write-Host '완료: dist\DaPDF.exe'
if ($Test) {
    & $gcc smoke_test.c vendor/bin/pdfium.dll -o dist/smoke_test.exe -std=c11 -O2 -Wall -Wextra -Werror @libs
    if ($LASTEXITCODE -ne 0) { throw '테스트 컴파일 실패' }
    & .\dist\smoke_test.exe
    if ($LASTEXITCODE -ne 0) { throw '스모크 테스트 실패' }
}
