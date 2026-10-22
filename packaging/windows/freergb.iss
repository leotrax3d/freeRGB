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
DisableDirPage=no
DisableProgramGroupPage=yes
UsePreviousTasks=yes
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
SetupIconFile=..\..\assets\icons\freergb.ico
UninstallDisplayIcon={app}\freergb-desktop.exe

[Languages]
Name: deutsch; MessagesFile: "compiler:Languages\German.isl"
Name: english; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: startmenu; Description: "Startmenü-Verknüpfungen erstellen"; GroupDescription: "Verknüpfungen:"
Name: desktopicon; Description: "Desktop-Verknüpfung erstellen"; GroupDescription: "Verknüpfungen:"; Flags: unchecked
Name: autostart; Description: "freeRGB bei der Windows-Anmeldung starten"; GroupDescription: "Autostart:"

[Files]
Source: "{#BinDir}\freergb-launcher.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb-desktop.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergbd.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb-mainboard.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb-keyboard-input.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb-mainboard-service.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "install-mainboard.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "remove-mainboard.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "mainboard-prerequisites.psm1"; DestDir: "{app}"; Flags: ignoreversion
Source: "mainboard-packages.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\freergb.exe"; Flags: dontcopy
Source: "{#BinDir}\freergb-mainboard-service.exe"; Flags: dontcopy
Source: "prepare-mainboard.ps1"; Flags: dontcopy
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\freeRGB"; Filename: "{app}\freergb-launcher.exe"; Tasks: startmenu
Name: "{group}\freeRGB Demo"; Filename: "{app}\freergb-launcher.exe"; Parameters: "--demo"; Tasks: startmenu
Name: "{group}\freeRGB Classic"; Filename: "{app}\freergb-launcher.exe"; Parameters: "--classic"; Tasks: startmenu
Name: "{group}\Uninstall freeRGB"; Filename: "{uninstallexe}"; Tasks: startmenu
Name: "{autodesktop}\freeRGB"; Filename: "{app}\freergb-launcher.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "freeRGB"; ValueData: """{app}\freergb-launcher.exe"" --daemon-only"; Tasks: autostart; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "freeRGB"; Flags: deletevalue; Check: RemoveOwnedAutostart

[InstallDelete]
Type: files; Name: "{group}\freeRGB.lnk"; Check: not WizardIsTaskSelected('startmenu')
Type: files; Name: "{group}\freeRGB Demo.lnk"; Check: not WizardIsTaskSelected('startmenu')
Type: files; Name: "{group}\freeRGB Classic.lnk"; Check: not WizardIsTaskSelected('startmenu')
Type: files; Name: "{group}\Uninstall freeRGB.lnk"; Check: not WizardIsTaskSelected('startmenu')
Type: files; Name: "{autodesktop}\freeRGB.lnk"; Check: not WizardIsTaskSelected('desktopicon')

[Run]
Filename: "{app}\freergb-launcher.exe"; Description: "Launch freeRGB"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\freergb.exe"; Parameters: "stop-installed-daemon --directory ""{app}"""; Flags: runhidden waituntilterminated skipifdoesntexist; RunOnceId: "StopDaemon"

[UninstallDelete]
Type: files; Name: "{app}\mainboard-provider.json"
Type: files; Name: "{app}\mainboard-install-result.json"
Type: filesandordirs; Name: "{app}\providers\lhm-0.9.6"

[Code]
function RemoveOwnedAutostart: Boolean;
var Value: String;
begin
  Result := False;
  if not WizardIsTaskSelected('autostart') then
    if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'freeRGB', Value) then
      Result := (CompareText(Value, '"' + ExpandConstant('{app}\freergb-launcher.exe') + '"') = 0) or
        (CompareText(Value, '"' + ExpandConstant('{app}\freergb-launcher.exe') + '" --daemon-only') = 0);
end;

#include "competitors.iss"
#include "mainboard-prerequisites.iss"

procedure InitializeWizard;
begin
  InitializeCompetitorsWizard;
  InitializeMainboardWizard;
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := ShouldSkipCompetitorsPage(PageID) or ((PageID = MainboardPage.ID) and WizardSilent);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then InstallSelectedMainboardComponents;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Code: Integer; Parameters: String;
begin
  if CurUninstallStep <> usUninstall then Exit;
  if not RegKeyExists(HKLM, 'SYSTEM\CurrentControlSet\Services\freeRGBMainboard') and
    not DirExists(ExpandConstant('{commonpf64}\freeRGB\MainboardBroker')) then Exit;
  Parameters := '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\remove-mainboard.ps1') +
    '" -InstallRoot "' + ExpandConstant('{app}') + '" -ConsentToRemove';
  if UninstallSilent then Parameters := Parameters + ' -Silent';
  if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Parameters,
    ExpandConstant('{app}'), SW_HIDE, ewWaitUntilTerminated, Code) then
    RaiseException('Der Mainboard-Dienst konnte nicht entfernt werden. Die Deinstallation wurde vor dem Entfernen der Programmdateien angehalten.');
  if Code <> 0 then
    RaiseException('Der Mainboard-Dienst wurde nicht entfernt oder die Administratorabfrage wurde abgebrochen. freeRGB bleibt für einen erneuten Versuch installiert. Details: Program Files\freeRGB\mainboard-maintenance-result.json');
end;

function NeedRestart: Boolean;
begin
  Result := MainboardRestart;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var ExitCode: Integer; Parameters: String;
begin
  Result := '';
  ExtractTemporaryFile('freergb-mainboard-service.exe');
  ExtractTemporaryFile('prepare-mainboard.ps1');
  Parameters := '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{tmp}\prepare-mainboard.ps1') +
    '" -MaintenanceBinary "' + ExpandConstant('{tmp}\freergb-mainboard-service.exe') + '"';
  if WizardSilent then Parameters := Parameters + ' -Silent';
  if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Parameters, '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
    Result := 'Mainboard-Wartung konnte nicht gestartet werden. Programmdateien bleiben erhalten.'
  else if ExitCode <> 0 then
    Result := 'Mainboard-Rückgabe nicht bestätigt oder UAC abgebrochen. Programmdateien bleiben erhalten.';
  if Result <> '' then Exit;
  if FileExists(ExpandConstant('{app}\freergbd.exe')) then begin
    ExtractTemporaryFile('freergb.exe');
    if not Exec(ExpandConstant('{tmp}\freergb.exe'), 'stop-installed-daemon --directory "' + ExpandConstant('{app}') + '"', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      Result := 'freeRGB konnte nicht geprüft werden. Bitte GUI und Daemon schließen.'
    else if ExitCode <> 0 then
      Result := 'Bitte freeRGB schließen und den Daemon beenden, dann erneut versuchen. Ältere Daemons müssen manuell geschlossen werden.';
  end;
end;
