; Bounce Tales installer (Inno Setup 6). Built by build_windows.ps1:
;   iscc /DAppVersion=1.0.0 packaging\windows\installer.iss
; Per-user install (no administrator rights needed): the game goes to
; %LOCALAPPDATA%\Programs\Bounce Tales, with a Start-menu entry, an
; optional desktop icon and an uninstaller. Paths are relative to this file.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{8C5A0E2B-6B4A-4F2E-9C1D-3B7E5A1F4D21}
AppName=Bounce Tales
AppVersion={#AppVersion}
AppVerName=Bounce Tales {#AppVersion}
AppPublisher=Jyotsan Hamal
AppPublisherURL=https://github.com/jyotsanh/bounce_tales
DefaultDirName={autopf}\Bounce Tales
DefaultGroupName=Bounce Tales
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\..\dist
OutputBaseFilename=bounce_tales-{#AppVersion}-windows-x64-setup
SetupIconFile=..\icon\bounce_tales.ico
UninstallDisplayIcon={app}\bounce_tales.exe
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\release\bounce_tales.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\Bounce Tales"; Filename: "{app}\bounce_tales.exe"
Name: "{group}\Uninstall Bounce Tales"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Bounce Tales"; Filename: "{app}\bounce_tales.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\bounce_tales.exe"; Description: "{cm:LaunchProgram,Bounce Tales}"; Flags: nowait postinstall skipifsilent
