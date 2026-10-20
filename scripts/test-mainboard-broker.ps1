$ErrorActionPreference = 'Stop'
$directory = [IO.Path]::GetFullPath("$PSScriptRoot/../target/broker-tests")
& "$PSScriptRoot/build-mainboard-helper.ps1" -OutputDirectory $directory
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$sources = @('Mainboard.cs','BrokerTransport.cs','BrokerCache.cs','BrokerService.cs','NativeControl.cs','tests/BrokerTests.cs') | ForEach-Object {
    (Resolve-Path -LiteralPath (Join-Path "$PSScriptRoot/../providers/mainboard" $_)).Path
}
$fixture = Join-Path $directory 'broker-tests.exe'
& $compiler /nologo /target:exe /main:BrokerTests /platform:x64 /warnaserror+ /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll "/out:$fixture" @sources
if ($LASTEXITCODE -ne 0) { throw 'Broker tests compilation failed.' }
& $fixture
if ($LASTEXITCODE -ne 0) { throw 'Broker acceptance failed.' }
& (Join-Path $directory 'freergb-mainboard.exe') --direct-protected-library
if ($LASTEXITCODE -ne 3) { throw 'Unprotected privileged worker was not rejected.' }
& (Join-Path $directory 'freergb-mainboard-service.exe') --unexpected
if ($LASTEXITCODE -ne 2) { throw 'Service accepted an unsupported argument.' }
$global:LASTEXITCODE = 0
