; Build via build-installer.ps1. Production setup and uninstall require elevation.
#ifndef AppVersion
  #error AppVersion must be supplied by build-installer.ps1
#endif
#ifndef SourceDir
  #error SourceDir must point to the staged release payload
#endif
#ifndef OutputDir
  #error OutputDir must be supplied by build-installer.ps1
#endif
#ifdef InstallerTest
  #define ProductName "DaPDF Installer Test"
  #define ProductId "{0A3AA95A-CCF1-4475-97F5-7C55F6A4DAE1}"
  #define InstallPrivileges "lowest"
  #define ProgId "DaPDF.InstallerTest.PDF"
  #define ApplicationKey "DaPDF-InstallerTest.exe"
#else
  #define ProductName "DaPDF"
  #define ProductId "{995B895F-B093-4C09-934D-0E457BD0AD98}"
  #define InstallPrivileges "admin"
  #define ProgId "DaPDF.PDF"
  #define ApplicationKey "DaPDF.exe"
#endif

[Setup]
AppId={{#ProductId}
AppName={#ProductName}
AppVersion={#AppVersion}
AppPublisher=yanghanjin-dev
AppPublisherURL=https://github.com/yanghanjin-dev/DaPDF
AppSupportURL=https://github.com/yanghanjin-dev/DaPDF/issues
DefaultDirName={autopf}\{#ProductName}
DefaultGroupName={#ProductName}
DisableProgramGroupPage=yes
PrivilegesRequired={#InstallPrivileges}
; No PrivilegesRequiredOverridesAllowed: production cannot switch to current-user mode.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputDir}
OutputBaseFilename=DaPDF-v{#AppVersion}-win-x64-setup
SetupIconFile=..\assets\branding\dapdf.ico
UninstallDisplayIcon={app}\DaPDF.exe
UninstallDisplayName={#ProductName} {#AppVersion}
Uninstallable=yes
CreateUninstallRegKey=yes
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
ChangesAssociations=yes
UsePreviousAppDir=yes
UsePreviousTasks=yes
DisableWelcomePage=no

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "바탕화면 바로가기 만들기"; Flags: unchecked
Name: "pdfopenwith"; Description: "PDF 파일의 '연결 프로그램' 목록에 DaPDF 등록"; Flags: unchecked

[Files]
Source: "{#SourceDir}\DaPDF.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\pdfium.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\VERSION"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\RELEASE-NOTES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE-PDFium.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#ProductName}\{#ProductName}"; Filename: "{app}\DaPDF.exe"; WorkingDir: "{app}"
Name: "{autoprograms}\{#ProductName}\{#ProductName} 제거"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#ProductName}"; Filename: "{app}\DaPDF.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
; HKA is HKLM in production (admin) and HKCU only in the isolated smoke-test build.
; Only register an Open With choice; never write PDF UserChoice or its default handler.
Root: HKA; Subkey: "Software\Classes\{#ProgId}"; ValueType: string; ValueData: "DaPDF PDF Document"; Flags: uninsdeletekey; Tasks: pdfopenwith
Root: HKA; Subkey: "Software\Classes\{#ProgId}\DefaultIcon"; ValueType: string; ValueData: "{app}\DaPDF.exe,0"; Tasks: pdfopenwith
Root: HKA; Subkey: "Software\Classes\{#ProgId}\shell\open\command"; ValueType: string; ValueData: """{app}\DaPDF.exe"" ""%1"""; Tasks: pdfopenwith
Root: HKA; Subkey: "Software\Classes\.pdf\OpenWithProgids"; ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: pdfopenwith
Root: HKA; Subkey: "Software\Classes\Applications\{#ApplicationKey}"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#ProductName}"; Flags: uninsdeletekey; Tasks: pdfopenwith
Root: HKA; Subkey: "Software\Classes\Applications\{#ApplicationKey}\shell\open\command"; ValueType: string; ValueData: """{app}\DaPDF.exe"" ""%1"""; Tasks: pdfopenwith
Root: HKA; Subkey: "Software\Classes\Applications\{#ApplicationKey}\SupportedTypes"; ValueType: string; ValueName: ".pdf"; ValueData: ""; Tasks: pdfopenwith

[Run]
Filename: "{app}\DaPDF.exe"; Description: "DaPDF 실행"; Flags: nowait postinstall skipifsilent runasoriginaluser unchecked

; No wildcard UninstallDelete: documents and other user-created files are preserved.
