# DaPDF Windows 설치 및 제거

## 사용자 동작

`releases/DaPDF-v0.1.4-win-x64-setup.exe`를 실행합니다. 설치 엔진이 UAC 관리자 권한을 요청하며, 모든 사용자용으로 설치합니다. 기본 경로는 `C:\Program Files\DaPDF`입니다. 시작 메뉴에는 실행과 제거 바로가기가 생성됩니다. 바탕화면 바로가기와 PDF 연결 프로그램 목록 등록은 선택 사항이며 PDF 기본 앱은 변경하지 않습니다.

Windows 설정의 앱 목록 또는 시작 메뉴의 `DaPDF 제거`를 통해 제거할 수 있습니다. 제거 프로그램도 관리자 권한을 요청합니다. 설치 프로그램이 기록한 파일·바로가기·등록 정보만 제거하며, 설치 경로 전체를 강제로 지우는 명령은 사용하지 않습니다. 사용자가 만든 문서가 남으면 설치 폴더도 남을 수 있습니다.

같은 AppId의 다음 설치는 기존 설치 경로와 제거 기록을 재사용합니다. 실행 중인 프로그램 파일은 Windows Restart Manager를 통해 종료 요청 대상이 됩니다. 설치·제거 전에 문서를 저장하고 DaPDF를 종료하세요. 프로그램 자체는 일반 사용자 권한으로 실행하며 설치·제거에만 관리자 권한을 요구합니다.

## 빌드

Inno Setup 6.7 이상 또는 호환 버전의 `ISCC.exe`가 필요합니다. `build-installer.ps1`은 `.tools/inno/ISCC.exe`, PATH 및 일반 설치 위치를 탐색합니다. 컴파일러는 저장소에 포함하지 않습니다.

```powershell
.\release.ps1
# 이미 준비된 payload에서 설치 EXE만 생성
.\build-installer.ps1 -StageDir .\dist\<staged-folder> -CompilerPath '<path>\ISCC.exe'
# 설치 EXE 없이 포터블만 생성
.\release.ps1 -SkipInstaller
```

`release.ps1`은 실행 중인 개발용 바이너리와 충돌하지 않도록 `target/distribution`에서 빌드합니다. 출력 설치 파일에는 현재 선택된 아이콘과 PDFium 및 전체 라이선스를 포함하며 SHA256 파일을 생성합니다. 현재 설치 파일은 코드 서명되지 않았습니다.

## 검증

```powershell
.\installer\test-install.ps1 -StageDir .\dist\<staged-folder>
```

테스트 빌드는 별도 AppId, 이름, 연결 프로그램 키를 사용하고, UAC 없이 실행할 수 있도록 테스트 빌드에 한해 사용자 설치로 제한합니다. 실제 배포 파일의 `PrivilegesRequired=admin`은 변경하지 않습니다. 테스트용 폴더는 저장소의 `target/installer-test/<unique-id>`에 생성합니다.

2026-11-16 검증 결과:

- PDF 및 도구 모음 회귀 테스트 2개 통과, 최적화 빌드 통과.
- 실제 배포용 관리자 설치 EXE 컴파일 성공.
- 격리된 테스트 빌드에서 설치 파일 해시·DLL·제거 프로그램·Windows 앱 등록·시작 메뉴·바탕화면·PDF 연결 프로그램 등록 확인.
- 동일 버전 재설치 후 사용자 문서 보존 확인.
- 제거 후 프로그램·DLL·제거 프로그램·앱 등록·바로가기·연결 프로그램 등록 제거 확인, 사용자 문서 보존 확인.
- 실제 Program Files/HKLM 설치와 UAC 승인 단계는 자동 실행하지 않았습니다. 배포 파일의 관리자 설치·제거 흐름은 별도 Windows 테스트 환경에서 최종 수동 확인해야 합니다.

Inno Setup의 외부 로더는 `asInvoker` 매니페스트를 사용하며, 관리자 권한 요청은 `PrivilegesRequired=admin`에 따라 내부 설치 엔진이 수행합니다. 로더의 매니페스트만으로 관리자 설치 여부를 판정하면 안 됩니다.

공식 문서: [관리자 권한 설정](https://jrsoftware.org/inno-setup/documentation/topic_setup_privilegesrequired.htm), [설치 로더의 권한 처리](https://jrsoftware.org/inno-setup/documentation/topic_securitymeasures.htm).
