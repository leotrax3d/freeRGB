param(
    [Parameter(Mandatory=$true)][ValidatePattern('^/lpc/[a-z0-9]+/[0-9]+/control/[0-6]$')][string]$ControlId,
    [Parameter(Mandatory=$true)][ValidatePattern('^/lpc/[a-z0-9]+/[0-9]+/fan/[0-6]$')][string]$RpmId,
    [Parameter(Mandatory=$true)][ValidatePattern('^/lpc/[a-z0-9]+/[0-9]+/temperature/[0-9]+$')][string]$TemperatureId,
    [switch]$ConsentToProbe,
    [switch]$Elevated
)
$ErrorActionPreference='Stop'
if(-not $ConsentToProbe){throw 'Explicit hardware commissioning consent required.'}
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
if(-not ([Security.Principal.WindowsPrincipal]$identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){
    if($Elevated){throw 'Elevation failed.'}
    $pilotArgs='-NoProfile -ExecutionPolicy Bypass -File "{0}" -ControlId {1} -RpmId {2} -TemperatureId {3} -ConsentToProbe -Elevated' -f $PSCommandPath,$ControlId,$RpmId,$TemperatureId
    $elevatedPilot=Start-Process -FilePath (Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe') -ArgumentList $pilotArgs -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    exit $elevatedPilot.ExitCode
}
$pilotRoot=Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'freeRGB'
$authorization=Join-Path $pilotRoot 'mainboard-control-authorization.json'
$pilotResult=Join-Path $pilotRoot 'mainboard-pilot-result.txt'
if(Test-Path -LiteralPath $authorization){throw 'Existing commissioning authorization must be reviewed first.'}
$pilotBinary=[IO.Path]::GetFullPath("$PSScriptRoot/../target/debug/freergb-cooling-pilot.exe")
$pilotHelper=[IO.Path]::GetFullPath("$PSScriptRoot/../target/release/freergb-mainboard.exe")
try {
    $payload=@{version=1;control_id=$ControlId;rpm_id=$RpmId;temperature_id=$TemperatureId;floor_percent=100;critical_c=80;expires_utc=[DateTime]::UtcNow.AddMinutes(2).ToString('o')} | ConvertTo-Json -Compress
    $stream=[IO.File]::Open($authorization,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
    try{$bytes=[Text.Encoding]::UTF8.GetBytes($payload);$stream.Write($bytes,0,$bytes.Length);$stream.Flush($true)}finally{$stream.Dispose()}
    $acl=Get-Acl -LiteralPath $authorization
    $acl.SetOwner([Security.Principal.SecurityIdentifier]::new('S-1-5-32-544'))
    Set-Acl -LiteralPath $authorization -AclObject $acl
    & $pilotBinary $pilotHelper *> $pilotResult
    if($LASTEXITCODE -ne 0){throw 'Rust native pilot failed; inspect protected mainboard-pilot-result.txt and recovery journal.'}
    $journal=Get-Content -Raw -LiteralPath (Join-Path $pilotRoot 'mainboard-control-recovery.json') | ConvertFrom-Json
    if($null -ne $journal.reservation){throw 'Pilot left unresolved native intent.'}
} catch {
    ($_ | Out-String) | Set-Content -LiteralPath $pilotResult -Encoding UTF8
    throw
} finally {
    if(Test-Path -LiteralPath $authorization){Remove-Item -LiteralPath $authorization -Force}
}
