var
  MainboardPage: TInputOptionWizardPage;
  MainboardRestart: Boolean;

procedure InitializeMainboardWizard;
begin
  MainboardPage := CreateInputOptionPage(wpSelectTasks, 'Mainboard-Lüfter und Sensoren',
    'Optionale Zusatzkomponenten installieren',
    'Optionale Mainboard-Sensoren: LibreHardwareMonitor 0.9.6, bei Bedarf PawnIO 2.2.0 und .NET 4.8.' + #13#10#13#10 +
    'Geprüfte offizielle Downloads; UAC, Herstellerdialoge und Neustart möglich. PawnIO: signierte Standardedition wählen.' + #13#10#13#10 +
    'Installiert oder aktualisiert auch den geschützten freeRGB-Sensordienst (Systemrechte, automatischer Dienststart). GUI/Daemon bleiben ohne Adminrechte. Experimentelle Lüfterregelung erfordert eine separate Bestätigung und UAC; RGB/ARGB separat. Ohne Auswahl keine Zusatzinstallation.', False, False);
  MainboardPage.Add('Benötigte Mainboard-Komponenten herunterladen und installieren (Internet erforderlich)');
  MainboardPage.Values[0] := False;
end;

procedure InstallSelectedMainboardComponents;
var Code: Integer; Parameters: String;
begin
  if WizardSilent or not MainboardPage.Values[0] then Exit;
  Parameters := '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\install-mainboard.ps1') +
    '" -InstallRoot "' + ExpandConstant('{app}') + '" -ConsentToInstall';
  if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Parameters,
    ExpandConstant('{app}'), SW_SHOWNORMAL, ewWaitUntilTerminated, Code) then begin
    MsgBox('Die optionalen Mainboard-Komponenten konnten nicht gestartet werden. Die normale freeRGB-Installation bleibt nutzbar. Setup kann später erneut ausgeführt werden.', mbError, MB_OK);
    Exit;
  end;
  if (Code = 3010) or (Code = 1641) then begin
    MainboardRestart := True;
    MsgBox('Die Zusatzkomponenten benötigen einen Windows-Neustart. Hinweise stehen in mainboard-install-result.json im Installationsordner. Falls die Vorbereitung noch unvollständig ist, danach Setup erneut mit der Mainboard-Option ausführen.', mbInformation, MB_OK);
  end else if Code <> 0 then
    MsgBox('Die Mainboard-Komponenten wurden nicht vollständig eingerichtet oder die Administratorabfrage wurde abgebrochen. Die normale freeRGB-Installation bleibt nutzbar. Details: mainboard-install-result.json im Installationsordner.', mbError, MB_OK);
end;
