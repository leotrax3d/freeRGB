param([string]$OutputDirectory = "$PSScriptRoot/../target/release")
$ErrorActionPreference = 'Stop'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if (-not (Test-Path -LiteralPath $compiler)) { throw '.NET Framework 4.8 x64 compiler is required.' }
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
$binary = Join-Path $outputPath 'freergb-mainboard.exe'
$source = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/Mainboard.cs").Path
$transport = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/BrokerTransport.cs").Path
$cache = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/BrokerCache.cs").Path
$service = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/BrokerService.cs").Path
$icon = (Resolve-Path -LiteralPath "$PSScriptRoot/../assets/icons/freergb.ico").Path
& $compiler /nologo /target:exe /main:Mainboard /platform:x64 /optimize+ /warnaserror+ /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll "/win32icon:$icon" "/out:$binary" $source $transport $cache $service
if ($LASTEXITCODE -ne 0) { throw 'Mainboard helper compilation failed.' }
$reply = & $binary --self-test | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $reply.version -ne 1 -or $reply.status -ne 'missing_library' -or $reply.hardware.Count -ne 0) { throw 'Mainboard helper serialization self-test failed.' }
Write-Output $binary
# Build the service candidate, but do not install/start it or ship it as an active component.
$serviceBinary = Join-Path $outputPath 'freergb-mainboard-service.exe'
& $compiler /nologo /target:exe /main:BrokerService /platform:x64 /optimize+ /warnaserror+ /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll "/win32icon:$icon" "/out:$serviceBinary" $source $transport $cache $service
if ($LASTEXITCODE -ne 0) { throw 'Mainboard broker compilation failed.' }
Write-Output $serviceBinary
