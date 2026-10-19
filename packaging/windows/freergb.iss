#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef BinDir
  #define BinDir "..\..\target\release"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\target\dist"
#endif

[Setup]
AppId={{A635E27A-ADBF-4DC2-8BF4-4237EC0C96ED}
AppName=freeRGB
AppVersion={#AppVersion}
AppPublisher=freeRGB contributors
AppPublisherURL=https://github.com/leotrax3d/FreeRGB
DefaultDirName={localappdata}\Programs\freeRGB
DefaultGroupName=freeRGB
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputDir}
OutputBaseFilename=freeRGB-{#AppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
UninstallDisplayIcon={app}\freergb-gui.exe

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: autostart; Description: "Start freeRGB when I sign in"; GroupDescription: "Startup:"; Flags: unchecked

[Files]
Source: "{#BinDir}\freergb-launcher.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergbd.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\freeRGB"; Filename: "{app}\freergb-launcher.exe"
Name: "{group}\freeRGB Demo"; Filename: "{app}\freergb-launcher.exe"; Parameters: "--demo"
Name: "{group}\Uninstall freeRGB"; Filename: "{uninstallexe}"
Name: "{autodesktop}\freeRGB"; Filename: "{app}\freergb-launcher.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "freeRGB"; ValueData: """{app}\freergb-launcher.exe"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "{app}\freergb-launcher.exe"; Description: "Launch freeRGB"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\freergb.exe"; Parameters: "ctl daemon.shutdown"; Flags: runhidden waituntilterminated skipifdoesntexist; RunOnceId: "StopDaemon"

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var ExitCode: Integer;
begin
  Result := '';
  if FileExists(ExpandConstant('{app}\freergb.exe')) then
    Exec(ExpandConstant('{app}\freergb.exe'), 'ctl daemon.shutdown', '', SW_HIDE, ewWaitUntilTerminated, ExitCode);
end;
