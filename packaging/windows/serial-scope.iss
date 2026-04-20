#define MyAppName "Serial Scope"
#define MyAppExeName "serial-scope.exe"
#define MyAppVersion GetVersionNumbersString(AddBackslash(SourcePath) + "..\\..\\target\\release\\" + MyAppExeName)
#define MyAppPublisher "Nitmi"
#define MyAppURL "https://github.com/Nitmi/serial-scope"

[Setup]
AppId={{E0434A18-5F8C-4F6C-90F0-5A9605B0A3D7}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist
OutputBaseFilename=serial-scope-windows-x86_64-setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\..\assets\app-icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"; Flags: checkedonce

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "启动 {#MyAppName}"; Flags: nowait postinstall skipifsilent
