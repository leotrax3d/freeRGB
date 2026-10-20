[Setup]
AppName=freeRGB installer model tests
AppVersion=1.0
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=lowest
OutputDir=..\..\target\installer-tests
OutputBaseFilename=installer-model-tests

[Code]
#include "competitors-model.iss"

procedure Require(Value: Boolean; Message: String);
begin
  if not Value then RaiseException(Message);
end;

function InitializeSetup: Boolean;
var FileName, Params: String;
begin
  Require(ProductFamily('SignalRgb 2.5', 'WhirlwindFX') = 'SignalRGB', 'SignalRGB detection');
  Require(ProductFamily('CORSAIR iCUE 5 Software', 'Corsair') = 'Corsair iCUE', 'iCUE detection');
  Require(ProductFamily('Razer Synapse', 'Razer Inc.') = 'Razer', 'Razer detection');
  Require(ProductFamily('Razer Mouse Driver', 'Razer') = '', 'Do not select arbitrary drivers');
  Require(ProductFamily('Razer Synapse', 'Unrelated Publisher') = '', 'Publisher mismatch');
  Require(ProductFamily('Unrelated application', '') = '', 'Unknown app');
  Require(SplitUninstaller('"C:\Program Files\Vendor\uninstall.exe" /uninstall', FileName, Params), 'Quoted path');
  Require(FileName = 'C:\Program Files\Vendor\uninstall.exe', 'Preserve quoted path');
  Require(Params = '/uninstall', 'Preserve arguments');
  Require(SplitUninstaller('MsiExec.exe /I{1234}', FileName, Params), 'MSI path');
  Require(Params = '/X{1234}', 'MSI uninstall rather than maintenance');
  Require(not SplitUninstaller('C:\Program Files\Vendor\uninstall.exe /x', FileName, Params), 'Ambiguous unquoted path');
  Require(not SplitUninstaller('"C:\Windows\System32\cmd.exe" /c anything', FileName, Params), 'No command shell');
  Require(not SplitUninstaller('"C:\Vendor\uninstall.exe', FileName, Params), 'Missing quote');
  Require(not SplitUninstaller('uninstall.exe', FileName, Params), 'No PATH search');
  Require(not SplitUninstaller('cleanup.ps1', FileName, Params), 'No scripts');
  Require(SaveStringToFile(ExpandConstant('{param:RESULT}'), 'PASS: installer matching and command parsing', False), 'Write test result');
  { Exit before setup mutates the system. No uninstaller is ever executed. }
  Result := False;
end;
