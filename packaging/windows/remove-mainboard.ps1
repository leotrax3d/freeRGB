param(
    [Parameter(Mandatory=$true)][string]$InstallRoot,
    [switch]$ConsentToRemove,
    [switch]$Silent
)
$ErrorActionPreference = 'Stop'
if (-not $ConsentToRemove) { throw 'Explicit owned-service removal consent is required.' }
$key = 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard'
$protected = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'freeRGB\MainboardBroker'
if (-not (Test-Path -LiteralPath $key) -and -not (Test-Path -LiteralPath $protected)) { exit 0 }
if ($Silent) { Write-Error 'Owned mainboard service requires interactive elevated removal; no silent UAC prompt was requested.'; exit 1 }
$root = (Resolve-Path -LiteralPath $InstallRoot).Path
$binary = Join-Path $root 'freergb-mainboard-service.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Broker maintenance executable is missing. Repair freeRGB Setup before removing the service.' }
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
try {
    $process = Start-Process -FilePath $binary -ArgumentList @('--remove','--consent',$sid) -WorkingDirectory (Join-Path $env:WINDIR 'System32') -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw 'Owned sensor service removal failed. See %ProgramFiles%\freeRGB\mainboard-maintenance-result.json. Shared PawnIO and .NET were not removed.' }
    exit 0
} catch { Write-Error $_.Exception.Message; exit 1 }
