#include "competitors-model.iss"

type
  TCompetitor = record
    Root: Integer;
    Key: String;
    Name: String;
    Command: String;
    Attempted: Boolean;
  end;
var
  Competitors: array of TCompetitor;
  CompetitorPage: TInputOptionWizardPage;
  CompetitorButton: TNewButton;
  CompetitorHint: TNewStaticText;

procedure ScanCompetitors(Root: Integer);
var Keys: TArrayOfString; I, J, N: Integer; Key, Name, Publisher, Command: String; Duplicate: Boolean;
begin
  if not RegGetSubkeyNames(Root, 'Software\Microsoft\Windows\CurrentVersion\Uninstall', Keys) then Exit;
  for I := 0 to GetArrayLength(Keys) - 1 do begin
    Key := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\' + Keys[I];
    if RegQueryStringValue(Root, Key, 'DisplayName', Name) then begin
      Publisher := '';
      RegQueryStringValue(Root, Key, 'Publisher', Publisher);
      if ProductFamily(Name, Publisher) <> '' then
        if RegQueryStringValue(Root, Key, 'UninstallString', Command) then begin
          Duplicate := False;
          for J := 0 to GetArrayLength(Competitors) - 1 do
            if (CompareText(Competitors[J].Name, Name) = 0) and (CompareText(Competitors[J].Command, Command) = 0) then Duplicate := True;
          if Duplicate then Continue;
          N := GetArrayLength(Competitors);
          if N >= 64 then Exit;
          SetArrayLength(Competitors, N + 1);
          Competitors[N].Root := Root;
          Competitors[N].Key := Key;
          Competitors[N].Name := Name;
          Competitors[N].Command := Command;
          Competitors[N].Attempted := False;
          CompetitorPage.Add(Name + ' (' + Publisher + ')');
          CompetitorPage.Values[N] := False;
        end;
    end;
  end;
end;

procedure UninstallSelectedCompetitors(Sender: TObject);
var I, Code: Integer; Names, Command, Name, FileName, Params: String;
begin
  Names := '';
  for I := 0 to GetArrayLength(Competitors) - 1 do
    if CompetitorPage.Values[I] and not Competitors[I].Attempted then
      Names := Names + #13#10 + '- ' + Competitors[I].Name;
  if Names = '' then begin
    MsgBox('Bitte zuerst ein Produkt auswählen. Bereits gestartete Deinstaller werden nicht erneut gestartet.', mbInformation, MB_OK);
    Exit;
  end;
  if MsgBox('Diese Hersteller-Deinstaller starten?' + Names + #13#10#13#10 +
    'Einstellungen und Funktionen dieser Produkte können verloren gehen. Herstellerdialoge und gegebenenfalls eine Windows-Administratorabfrage folgen. freeRGB löscht keine Treiber oder Reste selbst.',
    mbConfirmation, MB_YESNO or MB_DEFBUTTON2) <> IDYES then Exit;
  CompetitorButton.Enabled := False;
  WizardForm.NextButton.Enabled := False;
  WizardForm.BackButton.Enabled := False;
  try
    for I := 0 to GetArrayLength(Competitors) - 1 do
      if CompetitorPage.Values[I] and not Competitors[I].Attempted then begin
        if not MachineUninstaller(Competitors[I].Root) then begin
          MsgBox('Programmeinträge dieses Benutzers werden aus Sicherheitsgründen nicht erhöht ausgeführt. Bitte ' + Competitors[I].Name + ' über Windows-Einstellungen > Apps entfernen.', mbInformation, MB_OK);
          Continue;
        end;
        { Re-read the exact registered entry; never execute a stale changed command. }
        if not RegQueryStringValue(Competitors[I].Root, Competitors[I].Key, 'DisplayName', Name) or
           not RegQueryStringValue(Competitors[I].Root, Competitors[I].Key, 'UninstallString', Command) or
           (Name <> Competitors[I].Name) or (Command <> Competitors[I].Command) then begin
          MsgBox('Der Eintrag wurde geändert oder entfernt: ' + Competitors[I].Name + '. Bitte Windows-Einstellungen > Apps verwenden.', mbInformation, MB_OK);
          Continue;
        end;
        if not SplitUninstaller(Command, FileName, Params) or not FileExists(FileName) then begin
          MsgBox('Dieser Deinstallationsbefehl kann nicht eindeutig gestartet werden: ' + Name + '. Bitte Windows-Einstellungen > Apps verwenden.', mbInformation, MB_OK);
          Continue;
        end;
        CompetitorHint.Caption := 'Hersteller-Deinstaller läuft: ' + Name;
        WizardForm.Refresh;
        if ShellExec('', FileName, Params, ExtractFileDir(FileName), SW_SHOWNORMAL, ewWaitUntilTerminated, Code) then begin
          Competitors[I].Attempted := True;
          CompetitorPage.Values[I] := False;
          if (Code = 3010) or (Code = 1641) then
            MsgBox(Name + ': Der Hersteller meldet einen nötigen Neustart. Bitte vor der Hardware-Nutzung neu starten.', mbInformation, MB_OK)
          else if Code <> 0 then
            MsgBox(Name + ': Rückgabecode ' + IntToStr(Code) + '. Möglicherweise wurde abgebrochen. Bitte den Herstellerdialog bzw. Windows-Apps prüfen.', mbInformation, MB_OK);
        end else
          MsgBox('Deinstaller konnte nicht gestartet werden: ' + Name + '. Windows-Fehler ' + IntToStr(Code), mbError, MB_OK);
      end;
  finally
    CompetitorButton.Enabled := True;
    WizardForm.NextButton.Enabled := True;
    WizardForm.BackButton.Enabled := True;
    CompetitorHint.Caption := 'Deinstaller beendet/gestartet. Bitte in Windows-Apps prüfen, ob das Produkt entfernt wurde. Weiter installiert freeRGB.';
  end;
end;

procedure InitializeCompetitorsWizard;
begin
  CompetitorPage := CreateInputOptionPage(wpSelectDir, 'Andere RGB-Programme',
    'Optional: Konkurrenzprodukte deinstallieren',
    'Nichts ist vorausgewählt. Markieren Sie nur Produkte, die Sie entfernen möchten, und klicken Sie auf die Schaltfläche. Weiter allein entfernt nichts.', False, False);
  CompetitorPage.CheckListBox.Height := CompetitorPage.SurfaceHeight - CompetitorPage.CheckListBox.Top - ScaleY(94);
  CompetitorButton := TNewButton.Create(CompetitorPage);
  CompetitorButton.Parent := CompetitorPage.Surface;
  CompetitorButton.SetBounds(0, CompetitorPage.CheckListBox.Top + CompetitorPage.CheckListBox.Height + ScaleY(8), ScaleX(320), ScaleY(26));
  CompetitorButton.Caption := 'Ausgewählte Hersteller-Deinstaller starten';
  CompetitorButton.OnClick := @UninstallSelectedCompetitors;
  CompetitorHint := TNewStaticText.Create(CompetitorPage);
  CompetitorHint.Parent := CompetitorPage.Surface;
  CompetitorHint.SetBounds(0, CompetitorButton.Top + CompetitorButton.Height + ScaleY(6), CompetitorPage.SurfaceWidth, ScaleY(48));
  CompetitorHint.AutoSize := False;
  CompetitorHint.WordWrap := True;
  CompetitorHint.Caption := 'SignalRGB, Razer Synapse/Chroma/Central, Corsair iCUE, OpenRGB und Logitech G HUB werden anhand ihrer Windows-App-Einträge erkannt. Unbekannte Einträge bitte über Windows-Apps entfernen.';
  ScanCompetitors(HKCU64);
  ScanCompetitors(HKLM64);
  ScanCompetitors(HKCU32);
  ScanCompetitors(HKLM32);
  CompetitorButton.Enabled := GetArrayLength(Competitors) > 0;
end;

function ShouldSkipCompetitorsPage(PageID: Integer): Boolean;
begin
  Result := (PageID = CompetitorPage.ID) and ((GetArrayLength(Competitors) = 0) or WizardSilent);
end;
