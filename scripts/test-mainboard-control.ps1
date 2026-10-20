$ErrorActionPreference = 'Stop'
Import-Module "$PSScriptRoot/../packaging/windows/mainboard-prerequisites.psm1" -Force
$manifest = Read-MainboardPackages "$PSScriptRoot/../packaging/windows/mainboard-packages.json"
$directory = [IO.Path]::GetFullPath("$PSScriptRoot/../target/native-control-tests")
$cache = Join-Path $directory 'downloads'
$runtime = Join-Path $directory 'runtime'
$archive = Get-VerifiedPackage $manifest.packages.lhm $cache
Install-LhmRuntime $archive $runtime $manifest $cache
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$adapter = (Resolve-Path "$PSScriptRoot/../providers/mainboard/NativeControl.cs").Path
$tests = (Resolve-Path "$PSScriptRoot/../providers/mainboard/tests/NativeControlTests.cs").Path
$binary = Join-Path $directory 'native-control-tests.exe'
& $compiler /nologo /target:exe /define:NATIVE_CONTROL_TESTS /platform:x64 /warnaserror+ "/out:$binary" $adapter $tests
if ($LASTEXITCODE -ne 0) { throw 'Native control tests compilation failed.' }
& $binary $runtime
if ($LASTEXITCODE -ne 0) { throw 'Native control adapter acceptance failed.' }
$global:LASTEXITCODE = 0
