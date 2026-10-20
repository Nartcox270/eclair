; Installateur d'Éclair (Inno Setup 6). Compiler : ISCC.exe installer\eclair.iss
; Prérequis : cargo build --release

#define AppName "Éclair"
#define AppVersion "1.1.0"
#define AppExe "Eclair.exe"

[Setup]
AppId={{E5670DDD-5FAA-44BE-A009-49AB5F5F2D81}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppName}
VersionInfoVersion={#AppVersion}
VersionInfoDescription={#AppName} — optimiseur pour Windows
; Installation pour l'utilisateur courant : pas d'UAC pour installer (Éclair demande l'admin à son lancement).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
DefaultDirName={autopf}\Eclair
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
SetupIconFile=..\assets\eclair.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
OutputDir=..\dist
OutputBaseFilename=Eclair-Setup-{#AppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\docs\*.md"; DestDir: "{app}\docs"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{group}\Désinstaller {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; shellexec : laisse Windows afficher la demande d'administrateur d'Éclair.
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: postinstall nowait skipifsilent shellexec

[UninstallRun]
; Retire le lancement automatique du mode persistant (sans effet s'il n'existe pas).
Filename: "schtasks.exe"; Parameters: "/Delete /TN Eclair /F"; Flags: runhidden; RunOnceId: "RemoveAutostart"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\Eclair"
