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
AppName=LocalPlayer
AppVersion={#AppVersion}
AppVerName=LocalPlayer {#AppVersion}
AppPublisher=Okaza03
AppPublisherURL=https://github.com/Okaza03/LocalPlayer
AppSupportURL=https://github.com/Okaza03/LocalPlayer/issues
AppUpdatesURL=https://github.com/Okaza03/LocalPlayer/releases
DefaultDirName={autopf}\LocalPlayer
DefaultGroupName=LocalPlayer
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed={#WinArch}
ArchitecturesInstallIn64BitMode={#WinArch}
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
SetupIconFile=localplayer.ico
UninstallDisplayIcon={app}\localplayer.exe
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
Name: "{autoprograms}\LocalPlayer"; Filename: "{app}\localplayer.exe"
Name: "{autodesktop}\LocalPlayer"; Filename: "{app}\localplayer.exe"; Tasks: desktopicon

[Registry]
; "Open with > LocalPlayer" for videos, without taking over the default player
Root: HKA; Subkey: "Software\Classes\LocalPlayer.Video"; ValueType: string; ValueData: "Video"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\LocalPlayer.Video\DefaultIcon"; ValueType: string; ValueData: "{app}\localplayer.exe,0"
Root: HKA; Subkey: "Software\Classes\LocalPlayer.Video\shell\open\command"; ValueType: string; ValueData: """{app}\localplayer.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "LocalPlayer"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\shell\open\command"; ValueType: string; ValueData: """{app}\localplayer.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\.mp4\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mkv\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.webm\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.avi\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mov\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.m4v\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.flv\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.wmv\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mpg\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.mpeg\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.ogv\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.ts\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.3gp\OpenWithProgids"; ValueType: string; ValueName: "LocalPlayer.Video"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".mp4"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".mkv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".webm"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".avi"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".mov"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".m4v"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".flv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".wmv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".mpg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".mpeg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".ogv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".ts"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\localplayer.exe\SupportedTypes"; ValueType: string; ValueName: ".3gp"; ValueData: ""

[Run]
; also runs after a silent self-update, which is what restarts the app
Filename: "{app}\localplayer.exe"; Description: "{cm:LaunchProgram,LocalPlayer}"; Flags: nowait postinstall
