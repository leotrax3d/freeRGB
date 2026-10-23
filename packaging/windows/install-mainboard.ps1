param(
    [Parameter(Mandatory=$true)][string]$InstallRoot,
    [switch]$ConsentToInstall,
    [switch]$DownloadOnly,
    [string]$CacheDirectory,
    # SID of the user who started Setup (captured by the installer before elevation).
    [string]$OwnerSid
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'mainboard-prerequisites.psm1') -Force
if (-not $ConsentToInstall -and -not $DownloadOnly) { throw 'Explicit prerequisite installation consent is required.' }
$root = (Resolve-Path -LiteralPath $InstallRoot).Path
if (-not $DownloadOnly -and -not (Test-Path -LiteralPath (Join-Path $root 'freergbd.exe'))) { throw 'Not a freeRGB installation directory.' }
if (-not $CacheDirectory) { $CacheDirectory = Join-Path $env:TEMP ('freeRGB-prerequisites-' + [guid]::NewGuid().ToString('N')) }
$CacheDirectory = [IO.Path]::GetFullPath($CacheDirectory)
$manifest = Read-MainboardPackages (Join-Path $PSScriptRoot 'mainboard-packages.json')
$lhmPin = Get-LhmPin $manifest
$resultPath = Join-Path $root 'mainboard-install-result.json'
$result = @{version=1;status='failed';restart_required=$false;message='Installation did not complete.'}
# Machine-readable progress for the Setup progress page: PROGRESS:<percent>:<text>
function Step([int]$Percent, [string]$Text) { Write-Host ("PROGRESS:{0}:{1}" -f $Percent,$Text); [Console]::Out.Flush() }
try {
    Step 5 'Checking installed components'
    Write-Host 'freeRGB: optionale Mainboard-Komponenten werden geprüft und vorbereitet.'
    $netRelease = 0
    $netKey = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full' -ErrorAction SilentlyContinue
    if ($netKey) { $netRelease = [int]$netKey.Release }
    $pawn = Get-Service -Name PawnIO -ErrorAction SilentlyContinue
    $plan = Get-MainboardPlan $netRelease ($null -ne $pawn)
    Step 15 'Verifying the bundled LibreHardwareMonitor runtime'
    $libraryArchive = Get-VerifiedPackage $manifest.packages.lhm $CacheDirectory
    foreach ($notice in $manifest.notices) { $null = Get-VerifiedPackage $notice $CacheDirectory }
    $pawnInstaller = $null; $netInstaller = $null
    if ($plan.InstallPawnIO) { Step 25 'Downloading the signed PawnIO driver setup'; $pawnInstaller = Get-VerifiedPackage $manifest.packages.pawnio $CacheDirectory }
    else { Step 25 'PawnIO driver already installed' }
    if ($plan.InstallDotNet) { Step 35 'Downloading .NET Framework 4.8'; $netInstaller = Get-VerifiedPackage $manifest.packages.dotnet $CacheDirectory }
    else { Step 35 '.NET Framework 4.8 already installed' }
    if ($DownloadOnly) { Write-Host 'PASS: Downloads und Signaturen geprüft. Keine Installation oder Treiberänderung.'; exit 0 }
    if ($plan.InstallDotNet) {
        Step 45 'Installing .NET Framework 4.8'
        $code = Invoke-VerifiedInstaller $netInstaller $manifest.packages.dotnet '/passive /norestart'
        if ($code -in @(3010,1641)) {
            $result.status='restart_required';$result.restart_required=$true;$result.message='.NET installed. Restart Windows, then run freeRGB Setup again to finish the motherboard components.'
            exit 3010
        }
    }
    if ($plan.InstallPawnIO) { Step 55 'Installing the PawnIO driver (its own setup window may appear)'; $code = Invoke-VerifiedInstaller $pawnInstaller $manifest.packages.pawnio ''; if ($code -in @(3010,1641)) { $result.restart_required=$true } }
    $pawn = Get-Service -Name PawnIO -ErrorAction SilentlyContinue
    if (-not $pawn) { throw 'PawnIO installation was cancelled or the driver is unavailable.' }
    if ($pawn.Status -ne 'Running' -and -not $result.restart_required) {
        $start = Start-Process -FilePath (Join-Path $env:WINDIR 'System32\sc.exe') -ArgumentList 'start PawnIO' -Verb RunAs -WindowStyle Hidden -Wait -PassThru
        if ($start.ExitCode -notin @(0,1056)) { throw 'PawnIO driver did not start. Restart Windows or check its installer.' }
    }
    Step 70 'Installing the LibreHardwareMonitor runtime'
    $libraryRoot = Join-Path $root ($lhmPin.LibraryDirectory -replace '/','\')
    Install-LhmRuntime $libraryArchive $libraryRoot $manifest $CacheDirectory
    if ($result.restart_required) {
        $result.status='restart_required';$result.message='Prerequisites require a restart. Restart Windows, then run freeRGB Setup again to install the protected sensor service.'
        exit 3010
    }
    $broker = Join-Path $root 'freergb-mainboard-service.exe'
    if (-not (Test-Path -LiteralPath $broker -PathType Leaf)) { throw 'Protected broker maintenance executable is missing. Rerun current freeRGB Setup.' }
    Step 85 'Installing the protected mainboard service'
    # The original user is captured by Setup before elevation (another admin may
    # have supplied credentials); standalone runs fall back to the current user.
    $ownerSid = if ($OwnerSid) { $OwnerSid } else { [Security.Principal.WindowsIdentity]::GetCurrent().User.Value }
    if ($ownerSid -notmatch '^S-1-(5-21|12-1)-[0-9-]+$') { throw 'Invalid owner account for the protected service.' }
    $brokerHash = (Get-FileHash -LiteralPath $broker -Algorithm SHA256).Hash
    # Quote Windows arguments, including a user-supplied cache ending in a backslash.
    $quotedCache = '"' + ($CacheDirectory -replace '(\\+)$','$1$1') + '"'
    $arguments = '--install --consent {0} {1} "{2}" {3}' -f $ownerSid,$brokerHash,$libraryArchive,$quotedCache
    $maintenance = Start-Process -FilePath $broker -ArgumentList $arguments -WorkingDirectory (Join-Path $env:WINDIR 'System32') -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    if ($maintenance.ExitCode -ne 0) { throw 'Protected sensor service setup failed or was cancelled. Diagnostic: %ProgramFiles%\freeRGB\mainboard-maintenance-result.json. Existing service rollback is attempted automatically.' }
    # Fixed relative path: neither GUI config nor arbitrary executable arguments.
    $provider = @{version=1;enabled=$true;library=$lhmPin.LibraryDirectory} | ConvertTo-Json
    [IO.File]::WriteAllText((Join-Path $root 'mainboard-provider.json'),$provider)
    $result.status = if ($result.restart_required) { 'restart_required' } else { 'prepared' }
    $result.message='Protected mainboard sensor service installed. GUI and daemon remain unelevated. Fan control stays locked until the safety notice in the first-run wizard is accepted.'
    Step 100 'Motherboard components ready'
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
