param([switch]$VerifyRuntime)
$ErrorActionPreference = 'Stop'
$directory = [IO.Path]::GetFullPath("$PSScriptRoot/../target/deployment-tests")
& "$PSScriptRoot/build-mainboard-helper.ps1" -OutputDirectory $directory
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$sources = @('Mainboard.cs','BrokerTransport.cs','BrokerCache.cs','BrokerService.cs','BrokerDeployment.cs','NativeControlJournal.cs','tests/DeploymentTests.cs') | ForEach-Object {
    (Resolve-Path -LiteralPath (Join-Path "$PSScriptRoot/../providers/mainboard" $_)).Path
}
$manifest = (Resolve-Path -LiteralPath "$PSScriptRoot/../packaging/windows/mainboard-packages.json").Path
$fixture = Join-Path $directory 'deployment-tests.exe'
& $compiler /nologo /target:exe /main:DeploymentTests /platform:x64 /warnaserror+ /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll /reference:System.IO.Compression.dll /reference:System.IO.Compression.FileSystem.dll "/resource:$manifest,MainboardPackages" "/out:$fixture" @sources (Join-Path $directory 'BrokerBuild.cs')
if ($LASTEXITCODE -ne 0) { throw 'Deployment tests compilation failed.' }
if ($VerifyRuntime) {
    Import-Module "$PSScriptRoot/../packaging/windows/mainboard-prerequisites.psm1" -Force
    $packages = Read-MainboardPackages $manifest
    $cache = Join-Path $directory 'downloads'
    $archive = Get-VerifiedPackage $packages.packages.lhm $cache
    foreach ($notice in $packages.notices) { $null = Get-VerifiedPackage $notice $cache }
    & $fixture $archive $cache
} else { & $fixture }
if ($LASTEXITCODE -ne 0) { throw 'Deployment acceptance failed.' }
