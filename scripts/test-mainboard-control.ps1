$ErrorActionPreference = 'Stop'
Import-Module "$PSScriptRoot/../packaging/windows/mainboard-prerequisites.psm1" -Force
$manifest = Read-MainboardPackages "$PSScriptRoot/../packaging/windows/mainboard-packages.json"
$directory = [IO.Path]::GetFullPath("$PSScriptRoot/../target/native-control-tests")
$cache = Join-Path $directory 'downloads'
$runtime = Join-Path $directory 'runtime'
$archive = Get-VerifiedPackage $manifest.packages.lhm $cache
Install-LhmRuntime $archive $runtime $manifest $cache
$lhmPin = Write-LhmPinSource $manifest (Join-Path $directory 'LhmPin.g.cs')
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$adapter = (Resolve-Path "$PSScriptRoot/../providers/mainboard/NativeControl.cs").Path
$tests = (Resolve-Path "$PSScriptRoot/../providers/mainboard/tests/NativeControlTests.cs").Path
$sessionSources = @('NativeControlSession.cs','NativeSessionTransport.cs','NativeSessionClient.cs','NativeControlJournal.cs','JournaledNativeControl.cs','BrokerTransport.cs','BrokerService.cs','NativeProbeJob.cs','NativeControlBroker.cs','MainboardMaintenance.cs','BrokerCache.cs','tests/NativeSessionTests.cs','tests/NativeJournalTests.cs') | ForEach-Object { (Resolve-Path (Join-Path "$PSScriptRoot/../providers/mainboard" $_)).Path }
$binary = Join-Path $directory 'native-control-tests.exe'
& $compiler /nologo /target:exe /main:NativeControlTests /define:NATIVE_CONTROL_TESTS /platform:x64 /warnaserror+ /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll "/out:$binary" $adapter $tests $lhmPin @sessionSources
if ($LASTEXITCODE -ne 0) { throw 'Native control tests compilation failed.' }
& $binary $runtime
if ($LASTEXITCODE -ne 0) { throw 'Native control adapter acceptance failed.' }
$global:LASTEXITCODE = 0
