param([string]$OutputDirectory = "$PSScriptRoot/../target/release")
$ErrorActionPreference = 'Stop'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $output -Force | Out-Null
$binary = Join-Path $output 'freergb-keyboard-input.exe'
$source = (Resolve-Path "$PSScriptRoot/../providers/keyboard/KeyboardInput.cs").Path
$icon = (Resolve-Path "$PSScriptRoot/../assets/icons/freergb.ico").Path
& $compiler /nologo /target:exe /platform:x64 /optimize+ /warnaserror+ "/win32icon:$icon" "/out:$binary" $source
if ($LASTEXITCODE -ne 0) { throw 'Keyboard input helper compilation failed.' }
$reply = & $binary --self-test | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $reply.version -ne 1 -or $reply.kind -ne 'ready') { throw 'Keyboard helper serialization self-test failed.' }
Write-Output $binary
