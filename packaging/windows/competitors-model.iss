{ Shared by setup and the compiler-built regression harness. }
function ProductFamily(Name, Publisher: String): String;
begin
  Name := Lowercase(Trim(Name));
  Publisher := Lowercase(Publisher);
  Result := '';
  if ((Pos('signalrgb', Name) = 1) or (Pos('signal rgb', Name) = 1)) and
     ((Publisher = '') or (Pos('whirlwind', Publisher) > 0) or (Pos('signalrgb', Publisher) > 0)) then
    Result := 'SignalRGB'
  else if ((Pos('razer synapse', Name) = 1) or (Pos('razer chroma', Name) = 1) or (Name = 'razer central')) and
     ((Publisher = '') or (Pos('razer', Publisher) > 0)) then
    Result := 'Razer'
  else if ((Pos('corsair icue', Name) = 1) or (Pos('icue ', Name) = 1) or (Name = 'icue')) and
     ((Publisher = '') or (Pos('corsair', Publisher) > 0)) then
    Result := 'Corsair iCUE'
  else if (Name = 'openrgb') or (Pos('openrgb ', Name) = 1) then
    Result := 'OpenRGB'
  else if (Name = 'logitech g hub') and ((Publisher = '') or (Pos('logitech', Publisher) > 0)) then
    Result := 'Logitech G HUB';
end;

function SplitUninstaller(Command: String; var FileName, Params: String): Boolean;
var P: Integer;
begin
  Result := False;
  FileName := '';
  Params := '';
  Command := Trim(Command);
  if Command = '' then Exit;
  if Command[1] = '"' then begin
    Delete(Command, 1, 1);
    P := Pos('"', Command);
    if P = 0 then Exit;
    if (P < Length(Command)) and (Command[P + 1] <> ' ') then Exit;
    FileName := Copy(Command, 1, P - 1);
    Params := Trim(Copy(Command, P + 1, Length(Command)));
  end else begin
    P := Pos('.exe', Lowercase(Command));
    if P = 0 then Exit;
    if (P + 3 < Length(Command)) and (Command[P + 4] <> ' ') then Exit;
    FileName := Copy(Command, 1, P + 3);
    Params := Trim(Copy(Command, P + 4, Length(Command)));
    { Unquoted paths with spaces are ambiguous: do not guess. }
    if Pos(' ', FileName) > 0 then Exit;
  end;
  if Lowercase(ExtractFileExt(FileName)) <> '.exe' then Exit;
  if CompareText(FileName, 'msiexec.exe') = 0 then FileName := ExpandConstant('{sys}\msiexec.exe');
  { No shell, script interpreters, relative paths or PATH search. }
  if (Length(FileName) < 3) or ((Copy(FileName, 2, 2) <> ':\') and (Copy(FileName, 1, 2) <> '\\')) then Exit;
  if (CompareText(ExtractFileName(FileName), 'cmd.exe') = 0) or
     (CompareText(ExtractFileName(FileName), 'powershell.exe') = 0) or
     (CompareText(ExtractFileName(FileName), 'pwsh.exe') = 0) or
     (CompareText(ExtractFileName(FileName), 'rundll32.exe') = 0) or
     (CompareText(ExtractFileName(FileName), 'wscript.exe') = 0) or
     (CompareText(ExtractFileName(FileName), 'cscript.exe') = 0) then Exit;
  if CompareText(ExtractFileName(FileName), 'msiexec.exe') = 0 then
    if Uppercase(Copy(Params, 1, 2)) = '/I' then
      Params := '/X' + Copy(Params, 3, Length(Params));
  Result := True;
end;
