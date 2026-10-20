param(
    [Parameter(Mandatory=$true)][string]$InstallRoot,
    [switch]$ConsentToInstall,
    [switch]$DownloadOnly,
    [string]$CacheDirectory
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'mainboard-prerequisites.psm1') -Force
if (-not $ConsentToInstall -and -not $DownloadOnly) { throw 'Explicit prerequisite installation consent is required.' }
$root = (Resolve-Path -LiteralPath $InstallRoot).Path
if (-not $DownloadOnly -and -not (Test-Path -LiteralPath (Join-Path $root 'freergbd.exe'))) { throw 'Not a freeRGB installation directory.' }
if (-not $CacheDirectory) { $CacheDirectory = Join-Path $env:TEMP ('freeRGB-prerequisites-' + [guid]::NewGuid().ToString('N')) }
$CacheDirectory = [IO.Path]::GetFullPath($CacheDirectory)
$manifest = Read-MainboardPackages (Join-Path $PSScriptRoot 'mainboard-packages.json')
$resultPath = Join-Path $root 'mainboard-install-result.json'
$result = @{version=1;status='failed';restart_required=$false;message='Installation did not complete.'}
try {
    Write-Host 'freeRGB: optionale Mainboard-Komponenten werden geprüft und vorbereitet.'
    $netRelease = 0
    $netKey = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full' -ErrorAction SilentlyContinue
    if ($netKey) { $netRelease = [int]$netKey.Release }
    $pawn = Get-Service -Name PawnIO -ErrorAction SilentlyContinue
    $plan = Get-MainboardPlan $netRelease ($null -ne $pawn)
    $libraryArchive = Get-VerifiedPackage $manifest.packages.lhm $CacheDirectory
    foreach ($notice in $manifest.notices) { $null = Get-VerifiedPackage $notice $CacheDirectory }
    $pawnInstaller = $null; $netInstaller = $null
    if ($plan.InstallPawnIO) { $pawnInstaller = Get-VerifiedPackage $manifest.packages.pawnio $CacheDirectory }
    if ($plan.InstallDotNet) { $netInstaller = Get-VerifiedPackage $manifest.packages.dotnet $CacheDirectory }
    if ($DownloadOnly) { Write-Host 'PASS: Downloads und Signaturen geprüft. Keine Installation oder Treiberänderung.'; exit 0 }
    if ($plan.InstallDotNet) {
        $code = Invoke-VerifiedInstaller $netInstaller $manifest.packages.dotnet '/passive /norestart'
        if ($code -in @(3010,1641)) {
            $result.status='restart_required';$result.restart_required=$true;$result.message='.NET installed. Restart Windows, then rerun freeRGB Setup with the mainboard option.'
            exit 3010
        }
    }
    if ($plan.InstallPawnIO) { $code = Invoke-VerifiedInstaller $pawnInstaller $manifest.packages.pawnio ''; if ($code -in @(3010,1641)) { $result.restart_required=$true } }
    $pawn = Get-Service -Name PawnIO -ErrorAction SilentlyContinue
    if (-not $pawn) { throw 'PawnIO installation was cancelled or the driver is unavailable.' }
    if ($pawn.Status -ne 'Running' -and -not $result.restart_required) {
        $start = Start-Process -FilePath (Join-Path $env:WINDIR 'System32\sc.exe') -ArgumentList 'start PawnIO' -Verb RunAs -WindowStyle Hidden -Wait -PassThru
        if ($start.ExitCode -notin @(0,1056)) { throw 'PawnIO driver did not start. Restart Windows or check its installer.' }
    }
    $libraryRoot = Join-Path $root 'providers\lhm-0.9.6'
    Install-LhmRuntime $libraryArchive $libraryRoot $manifest $CacheDirectory
    if ($result.restart_required) {
        $result.status='restart_required';$result.message='Prerequisites require a restart. Restart Windows, then rerun Setup with the mainboard option to install the protected sensor service.'
        exit 3010
    }
    $broker = Join-Path $root 'freergb-mainboard-service.exe'
    if (-not (Test-Path -LiteralPath $broker -PathType Leaf)) { throw 'Protected broker maintenance executable is missing. Rerun current freeRGB Setup.' }
    # Capture the original user before UAC, including when another admin supplies credentials.
    $ownerSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $brokerHash = (Get-FileHash -LiteralPath $broker -Algorithm SHA256).Hash
    # Quote Windows arguments, including a user-supplied cache ending in a backslash.
    $quotedCache = '"' + ($CacheDirectory -replace '(\\+)$','$1$1') + '"'
    $arguments = '--install --consent {0} {1} "{2}" {3}' -f $ownerSid,$brokerHash,$libraryArchive,$quotedCache
    $maintenance = Start-Process -FilePath $broker -ArgumentList $arguments -WorkingDirectory (Join-Path $env:WINDIR 'System32') -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    if ($maintenance.ExitCode -ne 0) { throw 'Protected sensor service setup failed or was cancelled. Diagnostic: %ProgramFiles%\freeRGB\mainboard-maintenance-result.json. Existing service rollback is attempted automatically.' }
    # Fixed relative path: neither GUI config nor arbitrary executable arguments.
    $provider = @{version=1;enabled=$true;library='providers/lhm-0.9.6'} | ConvertTo-Json
    [IO.File]::WriteAllText((Join-Path $root 'mainboard-provider.json'),$provider)
    $result.status = if ($result.restart_required) { 'restart_required' } else { 'prepared' }
    $result.message='Protected mainboard sensor service installed. GUI and daemon remain unelevated. Fan control remains gated; physical channel labels and hardware compatibility still require validation.'
    Write-Host $result.message
    if ($result.restart_required) { exit 3010 }
    exit 0
} catch {
    $result.message=$_.Exception.Message
    Write-Host ('Mainboard-Komponenten nicht vollständig eingerichtet: ' + $result.message)
    exit 1
} finally {
    if (-not $DownloadOnly) { [IO.File]::WriteAllText($resultPath,($result | ConvertTo-Json)) }
}
