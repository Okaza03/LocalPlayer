; Windows installer. Built by `cargo xtask installer`, which passes:
;   /DAppVersion=X.Y.Z  /DSourceDir=<staged files>  /DOutputDir=...  /DOutputName=...
;
; Installs per user by default (no admin prompt), so the app can update itself
; by running the next installer with /SILENT.

#ifndef AppVersion
  #error Pass /DAppVersion=X.Y.Z
#endif
#ifndef Arch
  #define Arch "x86_64"
#endif
; x64 also runs on ARM64 Windows 11 (emulated); the aarch64 build only there
#if Arch == "aarch64"
  #define WinArch "arm64"
#else
  #define WinArch "x64compatible"
#endif

[Setup]
; never change the AppId: Windows uses it to find the existing install
AppId={{28C9BC2E-0255-4319-839A-77B2A0EB595F}
AppName=Valo
AppVersion={#AppVersion}
AppVerName=Valo {#AppVersion}
AppPublisher=Okaza03
AppPublisherURL=https://github.com/Okaza03/Valo
AppSupportURL=https://github.com/Okaza03/Valo/issues
AppUpdatesURL=https://github.com/Okaza03/Valo/releases
DefaultDirName={autopf}\Valo
DefaultGroupName=Valo
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed={#WinArch}
ArchitecturesInstallIn64BitMode={#WinArch}
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
SetupIconFile=valo.ico
UninstallDisplayIcon={app}\valo.exe
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{autoprograms}\Valo"; Filename: "{app}\valo.exe"
Name: "{autodesktop}\Valo"; Filename: "{app}\valo.exe"; Tasks: desktopicon

[Registry]
; "Open with > Valo" for videos, without taking over the default player
Root: HKA; Subkey: "Software\Classes\Valo.Video"; ValueType: string; ValueData: "Video"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Valo.Video\DefaultIcon"; ValueType: string; ValueData: "{app}\valo.exe,0"
Root: HKA; Subkey: "Software\Classes\Valo.Video\shell\open\command"; ValueType: string; ValueData: """{app}\valo.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "Valo"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\shell\open\command"; ValueType: string; ValueData: """{app}\valo.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\.mp4\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mkv\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.webm\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.avi\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mov\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.m4v\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.flv\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.wmv\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mpg\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mpeg\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.ogv\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.ts\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.3gp\OpenWithProgids"; ValueType: string; ValueName: "Valo.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".mp4"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".mkv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".webm"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".avi"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".mov"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".m4v"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".flv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".wmv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".mpg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".mpeg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".ogv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".ts"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\valo.exe\SupportedTypes"; ValueType: string; ValueName: ".3gp"; ValueData: ""

[Run]
; also runs after a silent self-update, which is what restarts the app
Filename: "{app}\valo.exe"; Description: "{cm:LaunchProgram,Valo}"; Flags: nowait postinstall
