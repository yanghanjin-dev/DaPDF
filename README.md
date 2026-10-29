# DaPDF v0.0.1

C11과 Win32 API로 작성한 Windows 64비트 PDF 뷰어입니다. PDF 해석 및 렌더링은 PDFium C API를 사용합니다.

## 다운로드

[최신 릴리즈 v0.0.1](https://github.com/yanghanjin-dev/DaPDF/releases/tag/v0.0.1)에서 `DaPDF-v0.0.1-windows-x64.zip`을 내려받아 압축을 풀고 `DaPDF.exe`를 실행하세요.

## 실행

`dist/DaPDF.exe`를 실행하세요. 같은 폴더의 `pdfium.dll`이 필요합니다.
PDF 열기 버튼 또는 파일 드래그 앤 드롭으로 문서를 엽니다. 명령줄 파일 경로도 지원합니다.

## 기능 및 단축키

- Ctrl+O: PDF 열기 (한글 경로 지원)
- 연속 스크롤: 모든 페이지를 세로로 이어서 표시 (화면에 보이는 페이지만 렌더링)
- 좌우 방향키 / Page Up·Down: 이전·다음 페이지로 이동
- Home / End: 첫·마지막 페이지
- + / − 또는 Ctrl+마우스 휠: 확대·축소
- F: 창 너비 맞춤 (가장 넓은 페이지 기준)
- R: 시계 방향 90도 회전
- 마우스 휠 / 위아래 방향키 / 스크롤바: 문서 스크롤, 현재 페이지 번호가 따라 바뀜

## 빌드

Windows x64, MinGW-w64 GCC, PowerShell이 필요합니다.

```powershell
.\setup.ps1
.\build.ps1          # 테스트까지: .\build.ps1 -Test
.\dist\DaPDF.exe
```

setup.ps1은 인터넷에서 최신 PDFium x64 바이너리와 공식 헤더, 라이선스를 내려받습니다.
다운로드 출처: https://github.com/bblanchon/pdfium-binaries
PDFium API: https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/fpdfview.h

## 범위

읽기 전용 뷰어입니다. 암호 입력, 텍스트 검색·선택, 인쇄, 편집은 구현하지 않았습니다.
파일은 최대 512MB이며 렌더링 버퍼는 페이지당 최대 2,500만 픽셀로 제한하고, 보이는 페이지와 앞뒤 한 페이지만 메모리에 유지합니다.
PDFium과 제3자 구성요소의 라이선스는 vendor와 배포 폴더의 LICENSE 파일을 확인하세요.

