param([Parameter(Mandatory=$true)][string]$Destination)
$ErrorActionPreference = 'Stop'
$version = '6.7.3'
$digest = '9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732'
if ((Test-Path -LiteralPath $Destination) -and (Get-ChildItem -LiteralPath $Destination -Force)) { throw 'Compiler destination must be empty.' }
$archive = Join-Path ([IO.Path]::GetTempPath()) ('freergb-inno-' + [guid]::NewGuid().ToString('N') + '.exe')
try {
    Invoke-WebRequest -Uri "https://github.com/jrsoftware/issrc/releases/download/is-6_7_3/innosetup-$version.exe" -OutFile $archive
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $digest) { throw 'Inno Setup checksum mismatch.' }
    if ((Get-AuthenticodeSignature -LiteralPath $archive).Status -ne 'Valid') { throw 'Inno Setup publisher signature is not valid.' }
    $process = Start-Process -FilePath $archive -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/CURRENTUSER',('/DIR="{0}"' -f $Destination)) -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw 'Pinned Inno Setup installation failed.' }
    $compiler = Join-Path $Destination 'ISCC.exe'
    if (-not (Test-Path -LiteralPath $compiler)) { throw 'Pinned compiler executable is missing.' }
    # Official ISCC PE resources can contain 0.0.0.0. Check the running
    # compiler's own preprocessor version instead of that placeholder resource.
    $probe = Join-Path ([IO.Path]::GetTempPath()) ('freergb-inno-probe-' + [guid]::NewGuid().ToString('N') + '.iss')
    try {
        [IO.File]::WriteAllText($probe, @'
#if Ver != EncodeVer(6, 7, 3)
  #error Pinned compiler version mismatch.
#endif
[Setup]
AppName=freeRGB compiler verification
AppVersion=1
DefaultDirName={tmp}\freeRGB-compiler-probe
CreateAppDir=no
Uninstallable=no
Output=no
'@)
        $output = & $compiler $probe 2>&1
        if ($LASTEXITCODE -ne 0) { throw "Pinned compiler verification failed: $output" }
    } finally { if (Test-Path -LiteralPath $probe) { Remove-Item -LiteralPath $probe } }
    Write-Output $compiler
} finally { if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive } }
