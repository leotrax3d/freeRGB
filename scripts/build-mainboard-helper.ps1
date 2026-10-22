param(
    [string]$OutputDirectory = "$PSScriptRoot/../target/release",
    [ValidateSet('Production','FixtureV1','FixtureV2','FixtureFailStart')][string]$Mode = 'Production'
)
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
$probe = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeProbeJob.cs").Path
$nativeControl = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeControl.cs").Path
$nativeEvidence = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeNctEvidence.cs").Path
$nativeWorker = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeControlWorker.cs").Path
$nativeBroker = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeControlBroker.cs").Path
$nativeMaintenance = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/MainboardMaintenance.cs").Path
$nativeSession = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeControlSession.cs").Path
$nativeTransport = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeSessionTransport.cs").Path
$nativeJournal = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeControlJournal.cs").Path
$journaledControl = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/JournaledNativeControl.cs").Path
$nativeClient = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/NativeSessionClient.cs").Path
$icon = (Resolve-Path -LiteralPath "$PSScriptRoot/../assets/icons/freergb.ico").Path
$helperOptions = @('/define:NATIVE_CONTROL_WORKER')
if ($Mode -ne 'Production') {
    $helperOptions = @('/define:BROKER_FIXTURE_WORKER;BROKER_FIXTURE_CLIENT_AUTH')
    if ($Mode -eq 'FixtureV2') { $helperOptions = @('/define:BROKER_FIXTURE_WORKER;BROKER_FIXTURE_V2;BROKER_FIXTURE_CLIENT_AUTH') }
}
& $compiler /nologo /target:exe /main:Mainboard /platform:x64 /optimize+ /warnaserror+ /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll @helperOptions "/win32icon:$icon" "/out:$binary" $source $transport $cache $service $probe $nativeControl $nativeEvidence $nativeWorker $nativeBroker $nativeMaintenance $nativeSession $nativeTransport $nativeJournal $journaledControl
if ($LASTEXITCODE -ne 0) { throw 'Mainboard helper compilation failed.' }
$reply = & $binary --self-test | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $reply.version -ne 1 -or $reply.status -ne 'missing_library' -or $reply.hardware.Count -ne 0) { throw 'Mainboard helper serialization self-test failed.' }
Write-Output $binary
# Maintenance requires explicit consent and elevation; building never installs a service.
$serviceBinary = Join-Path $outputPath 'freergb-mainboard-service.exe'
$deployment = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/BrokerDeployment.cs").Path
$manifest = (Resolve-Path -LiteralPath "$PSScriptRoot/../packaging/windows/mainboard-packages.json").Path
$identity = Join-Path $outputPath 'BrokerBuild.cs'
$helperHash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText($identity, ('internal static class BrokerBuild {{ internal const long HelperSize = {0}; internal const string HelperSha256 = "{1}"; }}' -f (Get-Item -LiteralPath $binary).Length, $helperHash))
$serviceDefine = '/define:BROKER_INSTALLER'
if ($Mode -ne 'Production') { $serviceDefine = '/define:BROKER_INSTALLER;BROKER_FIXTURE_CLIENT_AUTH' }
if ($Mode -eq 'FixtureFailStart') { $serviceDefine = '/define:BROKER_INSTALLER;BROKER_FIXTURE_FAIL_START' }
& $compiler /nologo /target:exe $serviceDefine /main:BrokerService /platform:x64 /optimize+ /warnaserror+ /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll /reference:System.IO.Compression.dll /reference:System.IO.Compression.FileSystem.dll "/resource:$manifest,MainboardPackages" "/win32icon:$icon" "/out:$serviceBinary" $source $transport $cache $service $probe $nativeBroker $nativeMaintenance $deployment $identity $nativeClient $nativeJournal
if ($LASTEXITCODE -ne 0) { throw 'Mainboard broker compilation failed.' }
Write-Output $serviceBinary
