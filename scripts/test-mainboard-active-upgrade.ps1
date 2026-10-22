param([switch]$ConsentToProbe,[switch]$Elevated)
$ErrorActionPreference='Stop'
if(-not $ConsentToProbe){throw 'Explicit live CHA_FAN2 maintenance acceptance consent required.'}
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
if(-not ([Security.Principal.WindowsPrincipal]$identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){
    if($Elevated){throw 'Elevation failed.'}
    $upgradeArgs='-NoProfile -ExecutionPolicy Bypass -File "{0}" -ConsentToProbe -Elevated' -f $PSCommandPath
    $upgradeElevated=Start-Process -FilePath (Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe') -ArgumentList $upgradeArgs -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    exit $upgradeElevated.ExitCode
}
# Deliberately bench-specific: never infer a different user's physical fan mapping.
$upgradeRoot=Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'freeRGB'
$upgradeResult=Join-Path $upgradeRoot 'mainboard-active-upgrade-result.txt'
$upgradeJournal=Join-Path $upgradeRoot 'mainboard-control-recovery.json'
$upgradePilot=$null
try {
    $oldService=(Get-CimInstance Win32_Service -Filter "Name='freeRGBMainboard'").ProcessId
    if(-not $oldService){throw 'Expected already installed running service.'}
    $pilotScript=Join-Path $PSScriptRoot 'test-mainboard-pilot.ps1'
    $pilotArguments='-NoProfile -ExecutionPolicy Bypass -File "{0}" -ControlId /lpc/nct6701d/0/control/2 -RpmId /lpc/nct6701d/0/fan/2 -TemperatureId /lpc/nct6701d/0/temperature/1 -Mode 80 -ConsentToProbe -Elevated' -f $pilotScript
    $upgradePilot=Start-Process -FilePath (Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe') -ArgumentList $pilotArguments -WindowStyle Hidden -PassThru
    $armDeadline=[DateTime]::UtcNow.AddSeconds(12)
    do {
        Start-Sleep -Milliseconds 100
        $armed=(Get-Content -Raw -LiteralPath $upgradeJournal | ConvertFrom-Json).reservation
    } while($null -eq $armed -and [DateTime]::UtcNow -lt $armDeadline)
    if($null -eq $armed -or $armed.sensor_id -ne '/lpc/nct6701d/0/control/2'){throw 'Expected exact selected live channel intent; maintenance not attempted.'}
    Start-Sleep -Milliseconds 1000
    $still=(Get-Content -Raw -LiteralPath $upgradeJournal | ConvertFrom-Json).reservation
    if($null -eq $still -or $still.id -ne $armed.id){throw 'Control ended before maintenance; no active-session acceptance.'}
    & "$PSScriptRoot/../packaging/windows/install-mainboard.ps1" -InstallRoot "$PSScriptRoot/../target/release" -ConsentToInstall -CacheDirectory "$PSScriptRoot/../target/native-control-tests/downloads"
    $install=Get-Content -Raw -LiteralPath "$PSScriptRoot/../target/release/mainboard-install-result.json" | ConvertFrom-Json
    if($install.status -ne 'prepared'){throw 'Active maintenance installation failed.'}
    $service=Get-CimInstance Win32_Service -Filter "Name='freeRGBMainboard'"
    if($service.State -ne 'Running' -or $service.ProcessId -eq $oldService){throw 'Expected replaced service lifetime.'}
    if($null -ne (Get-Content -Raw -LiteralPath $upgradeJournal | ConvertFrom-Json).reservation){throw 'Replacement left unresolved native intent.'}
    $ready=Get-Content -Raw -LiteralPath (Join-Path $upgradeRoot 'mainboard-maintenance-ready.json') | ConvertFrom-Json
    if(-not $ready.ready){throw 'No verified service-maintenance acknowledgement.'}
    if(-not $upgradePilot.WaitForExit(20000)){throw 'Interrupted commissioning caller did not finish.'}
    'PASS: active 80 percent CHA_FAN2 session drained, verified journal cleared, admission closed, service payload replaced and new service running. Same-version replacement, not release-to-release update acceptance.' | Set-Content -LiteralPath $upgradeResult -Encoding UTF8
} catch {
    ($_ | Out-String) | Set-Content -LiteralPath $upgradeResult -Encoding UTF8
    throw
} finally {
    if($upgradePilot){$upgradePilot.Dispose()}
}
