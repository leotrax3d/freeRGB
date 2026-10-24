// Automatic motherboard components (docs/SETUP_WIZARD.md). Setup is elevated:
// .NET 4.8 (only if missing), PawnIO (only if missing; signed official setup is
// downloaded and verified), the bundled LibreHardwareMonitor runtime and the
// protected service are installed without a component page. A progress page
// shows each step. Silent installs skip this unless /WITHMAINBOARD is given.
#include "installer-policy.iss"
var
  MainboardProgress: TOutputProgressWizardPage;
  MainboardRestart: Boolean;
  MainboardError: String;

procedure InitializeMainboardWizard;
begin
  MainboardProgress := CreateOutputProgressPage('Setting up motherboard fan and sensor support',
    'freeRGB checks and installs the required components. Already installed components are skipped.');
end;

function MainboardRequested: Boolean;
var I: Integer;
begin
  Result := not WizardSilent;
  if Result then Exit;
  for I := 1 to ParamCount do
    if IsMainboardOptIn(ParamStr(I)) then begin Result := True; Exit; end;
end;

// Lines "PROGRESS:<percent>:<text>" from install-mainboard.ps1 drive the page.
procedure MainboardLog(const S: String; const Error, FirstLine: Boolean);
var Rest, Percent: String; Split: Integer;
begin
  Log('mainboard: ' + S);
  if Copy(S, 1, 9) <> 'PROGRESS:' then Exit;
  Rest := Copy(S, 10, Length(S));
  Split := Pos(':', Rest);
  if Split = 0 then Exit;
  Percent := Copy(Rest, 1, Split - 1);
  MainboardProgress.SetText(Copy(Rest, Split + 1, Length(Rest)), '');
  MainboardProgress.SetProgress(StrToIntDef(Percent, 0), 100);
end;

// SID of the user who started Setup, captured as that user (not the elevated one).
function OriginalUserSid: String;
var Code: Integer; Output: AnsiString; Target: String;
begin
  Result := '';
  Target := ExpandConstant('{tmp}\original-user-sid.txt');
  if ExecAsOriginalUser(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    '-NoProfile -Command "[IO.File]::WriteAllText(''' + Target + ''',[Security.Principal.WindowsIdentity]::GetCurrent().User.Value)"',
    '', SW_HIDE, ewWaitUntilTerminated, Code) and (Code = 0) and LoadStringFromFile(Target, Output) then
    Result := Trim(String(Output));
end;

procedure InstallMainboardComponents;
var Code: Integer; Parameters, Owner: String;
begin
  if not MainboardRequested then Exit;
  MainboardError := '';
  Owner := OriginalUserSid;
  if Owner = '' then begin
    MsgBox('The original signed-in user could not be identified. Motherboard components were not installed. Lighting still works; rerun Setup from the intended user account.', mbError, MB_OK);
    Exit;
  end;
  Parameters := '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\install-mainboard.ps1') +
    '" -InstallRoot "' + ExpandConstant('{app}') + '" -ConsentToInstall -CacheDirectory "' + ExpandConstant('{tmp}\prerequisites') + '"';
  if Owner <> '' then Parameters := Parameters + ' -OwnerSid ' + Owner;
  MainboardProgress.Show;
  try
    MainboardProgress.SetText('Checking installed components', '');
    MainboardProgress.SetProgress(0, 100);
    if not ExecAndLogOutput(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Parameters,
      ExpandConstant('{app}'), SW_HIDE, ewWaitUntilTerminated, Code, @MainboardLog) then
      MainboardError := 'The motherboard components could not be started.'
    else if (Code = 3010) or (Code = 1641) then
      MainboardRestart := True
    else if Code <> 0 then
      MainboardError := 'The motherboard components were not fully set up (for example the driver setup or a permission prompt was cancelled, or a download failed).';
  finally
    MainboardProgress.Hide;
  end;
  if MainboardError <> '' then
    MsgBox(MainboardError + #13#10#13#10 +
      'freeRGB lighting works without them. Fan control stays unavailable until Setup is run again.' + #13#10 +
      'Details: mainboard-install-result.json in the installation folder and the Setup log.', mbError, MB_OK)
  else if MainboardRestart then
    MsgBox('Windows needs a restart to finish the driver installation. Afterwards run freeRGB Setup again to complete the protected fan service.', mbInformation, MB_OK);
end;
