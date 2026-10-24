{ Shared with the no-install compiler regression harness. }
function IsMainboardOptIn(Argument: String): Boolean;
begin
  Result := CompareText(Argument, '/WITHMAINBOARD') = 0;
end;
