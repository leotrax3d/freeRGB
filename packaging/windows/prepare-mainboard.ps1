param([Parameter(Mandatory=$true)][string]$MaintenanceBinary,[switch]$Silent)
$ErrorActionPreference = 'Stop'
# Runs before Setup replaces any application file. The protected service stays
# installed and owns admission; cancellation leaves it safely in maintenance.
$key = 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard'
$protected = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'freeRGB\MainboardBroker'
if (-not (Test-Path -LiteralPath $key) -and -not (Test-Path -LiteralPath $protected)) { exit 0 }
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$elevated = ([Security.Principal.WindowsPrincipal]$identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($Silent -and -not $elevated) { throw 'Silent mainboard maintenance requires elevation.' }
$binary = (Resolve-Path -LiteralPath $MaintenanceBinary).Path
$options = @{FilePath=$binary;ArgumentList=@('--prepare-maintenance','--consent',$identity.User.Value);WorkingDirectory=(Join-Path $env:WINDIR 'System32');WindowStyle='Hidden';Wait=$true;PassThru=$true}
if (-not $elevated) { $options.Verb='RunAs' }
try {
    $process = Start-Process @options
    if ($process.ExitCode -ne 0) { throw 'Mainboard maintenance refused; preserve installed binaries and recovery evidence.' }
} catch { Write-Error $_.Exception.Message; exit 1 }
exit 0
