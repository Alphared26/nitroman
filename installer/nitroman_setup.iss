; Inno Setup Script for NitroMan - Acer Nitro Hardware Control Center
; Strictly Zero Emojis

#define MyAppName "NitroMan"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "HermesAI"
#define MyAppURL "https://github.com/nitroman"
#define MyAppExeName "NitroMan.exe"

[Setup]
AppId={{8B841F9D-7E3C-4C82-9B5A-01D78F45DE33}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=NitroMan-Setup-v1.0.0
SetupIconFile=app_icon.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\app_icon.ico
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "Launch NitroMan automatically at Windows logon (Elevated Task Scheduler)"; GroupDescription: "Startup Options:"

[Files]
; Core Application Files
Source: "bundled\app\NitroMan.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "bundled\app\nitroman-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "bundled\app\nitroman-ec.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "bundled\app\TsDotNetLib.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "app_icon.ico"; DestDir: "{app}"; Flags: ignoreversion

; NitroSense Service Files (Protected Ring-0 EC Named Pipe Provider)
Source: "bundled\service_nitrosense\*"; DestDir: "{commonpf}\Acer\NitroSense Service"; Flags: ignoreversion recursesubdirs createallsubdirs uninsneveruninstall
Source: "bundled\service_nitrosense\*"; DestDir: "{app}\service_nitrosense"; Flags: ignoreversion recursesubdirs createallsubdirs

; Care Center WMI Bridge Utilities (Battery 80% Conservation Mode Provider)
Source: "bundled\service_carecenter\*"; DestDir: "{commonpf32}\Acer\Care Center"; Flags: ignoreversion uninsneveruninstall

[Registry]
Root: HKLM; Subkey: "SOFTWARE\OEM\NitroSense"; Flags: noerror
Root: HKLM; Subkey: "SOFTWARE\OEM\NitroSense\KeyAssignment"; Flags: noerror

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app_icon.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app_icon.ico"; Tasks: desktopicon

[Run]
; Ensure PSSvc service is created and running
Filename: "{cmd}"; Parameters: "/c sc query PSSvc >nul 2>&1 || sc create PSSvc binPath= ""{commonpf}\Acer\NitroSense Service\PSSvc.exe"" start= auto DisplayName= ""Predator Service"""; Flags: runhidden
Filename: "{cmd}"; Parameters: "/c sc config PSSvc start= auto && sc start PSSvc"; Flags: runhidden

; Configure Windows Task Scheduler to run NitroMan elevated at logon (No UAC prompts)
Filename: "schtasks.exe"; Parameters: "/create /tn ""NitroMan"" /tr """"{app}\{#MyAppExeName}"""" /sc onlogon /rl highest /f"; Flags: runhidden; Tasks: autostart

; Launch NitroMan after installation with elevated privileges
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent shellexec

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM {#MyAppExeName}"; Flags: runhidden; RunOnceId: "StopNitroMan"
Filename: "schtasks.exe"; Parameters: "/delete /tn NitroMan /f"; Flags: runhidden; RunOnceId: "RemoveScheduledTask"

[Code]
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM {#MyAppExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := True;
end;
