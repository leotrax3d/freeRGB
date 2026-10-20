param([switch]$VerifyDownloads)
$ErrorActionPreference = 'Stop'
Import-Module "$PSScriptRoot/../packaging/windows/mainboard-prerequisites.psm1" -Force
function Require($Condition,[string]$Message) { if (-not $Condition) { throw $Message } }
$manifest = Read-MainboardPackages "$PSScriptRoot/../packaging/windows/mainboard-packages.json"
foreach ($net in @(0,461808,528040,534296)) {
    foreach ($pawn in @($false,$true)) {
        $plan = Get-MainboardPlan $net $pawn
        Require ($plan.InstallDotNet -eq ($net -lt 528040)) 'Framework detection boundary failed.'
        Require ($plan.InstallPawnIO -eq (-not $pawn)) 'Existing driver must not be reinstalled by default.'
        Require $plan.InstallLibrary 'Runtime deployment must be planned.'
    }
}
$root = [IO.Path]::GetFullPath("$PSScriptRoot/../target/prerequisite-tests")
New-Item -ItemType Directory -Path $root -Force | Out-Null
$bootstrap = (Resolve-Path "$PSScriptRoot/../packaging/windows/install-mainboard.ps1").Path
$powershell = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
$gate = Start-Process -FilePath $powershell -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',('"{0}"' -f $bootstrap),'-InstallRoot',('"{0}"' -f $root)) -WindowStyle Hidden -Wait -PassThru -RedirectStandardError (Join-Path $root 'consent-gate-error.log') -RedirectStandardOutput (Join-Path $root 'consent-gate-output.log')
Require ($gate.ExitCode -ne 0) 'Bootstrap must reject missing installation consent.'
Require (-not (Test-Path (Join-Path $root 'mainboard-provider.json'))) 'Missing consent must not create activation state.'
$fake = Join-Path $root 'fake.exe'
[IO.File]::WriteAllText($fake,'Not an executable; must never be launched.')
$package = [pscustomobject]@{file='fake.exe';size=(Get-Item $fake).Length;sha256=(Get-FileHash $fake).Hash.ToLowerInvariant();signer='Microsoft Corporation';url='https://github.com/unused'}
Require (Test-PackageHash $fake $package) 'Known test hash rejected.'
$rejected = $false
try { $null = Get-VerifiedPackage $package $root } catch { $rejected = $_.Exception.Message -like '*Signature*' }
Require $rejected 'Unsigned payload must be rejected even when its content hash matches.'
[IO.File]::AppendAllText($fake,'changed')
Require (-not (Test-PackageHash $fake $package)) 'Tampered content accepted.'
$invalid = Get-Content "$PSScriptRoot/../packaging/windows/mainboard-packages.json" -Raw | ConvertFrom-Json
$invalid.packages.lhm.url='http://github.com/insecure'
$invalidPath=Join-Path $root 'invalid.json'; $invalid | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $invalidPath
$rejected=$false;try {$null=Read-MainboardPackages $invalidPath} catch {$rejected=$true}
Require $rejected 'Insecure download manifest accepted.'
Write-Output 'PASS: consent gate, prerequisite plans, pinned manifest, tamper and unsigned-payload rejection. No system changes.'
if ($VerifyDownloads) {
    $cache=Join-Path $root 'downloads'
    foreach ($entry in @($manifest.packages.lhm,$manifest.packages.pawnio,$manifest.packages.dotnet) + @($manifest.notices)) {
        $null=Get-VerifiedPackage $entry $cache
    }
    $runtime=Join-Path $root 'runtime'
    Install-LhmRuntime (Join-Path $cache $manifest.packages.lhm.file) $runtime $manifest $cache
    Require (Test-Path (Join-Path $runtime 'LibreHardwareMonitorLib.dll')) 'Runtime was not staged.'
    Require (-not (Test-Path (Join-Path $runtime 'LibreHardwareMonitor.exe'))) 'Reference GUI must not be installed.'
    Require (Test-Path (Join-Path $runtime 'PawnIO.Modules-LGPL-2.1.txt')) 'Embedded module license missing.'
    Require (Test-Path (Join-Path $runtime 'SOURCES.txt')) 'Source provenance missing.'
    Write-Output 'PASS: official downloads, hashes, signatures, runtime staging and notices. No installers executed; no driver loaded.'
}
