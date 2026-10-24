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
    if (-not (Test-Path -LiteralPath $compiler) -or (Get-Item -LiteralPath $compiler).VersionInfo.FileVersion -notmatch '^6\.7\.3(?:\.|$)') { throw 'Pinned compiler version mismatch.' }
    Write-Output $compiler
} finally { if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive } }
