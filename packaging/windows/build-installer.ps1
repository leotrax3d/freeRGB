param([string]$Compiler, [string]$BinDir = "$PSScriptRoot/../../target/release", [string]$OutputDir = "$PSScriptRoot/../../target/dist")
$ErrorActionPreference = 'Stop'
if (-not $Compiler) {
    $candidates = @("${env:ProgramFiles(x86)}/Inno Setup 6/ISCC.exe", "$env:ProgramFiles/Inno Setup 7/ISCC.exe", "$env:ProgramFiles/Inno Setup 6/ISCC.exe")
    $Compiler = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $Compiler) { throw 'Inno Setup compiler not found. Pass -Compiler PATH or build using the GitHub Package workflow.' }
$binPath = (Resolve-Path -LiteralPath $BinDir).Path
& "$PSScriptRoot/../../scripts/build-desktop.ps1" -OutputDirectory $binPath
& "$PSScriptRoot/../../scripts/build-mainboard-helper.ps1" -OutputDirectory $binPath -Mode Production
& "$PSScriptRoot/../../scripts/build-keyboard-helper.ps1" -OutputDirectory $binPath
$outputPath = [System.IO.Path]::GetFullPath($OutputDir)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
$metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$version = ($metadata.packages | Where-Object { $_.name -eq 'freergb-gui' }).version
if ($env:GITHUB_REF_TYPE -eq 'tag' -and $env:GITHUB_REF_NAME -ne "v$version") { throw 'Release tag must match the Cargo workspace version.' }
Import-Module "$PSScriptRoot/mainboard-prerequisites.psm1" -Force
$packages = Read-MainboardPackages "$PSScriptRoot/mainboard-packages.json"
$lhmPin = Get-LhmPin $packages
# Bundle the verified LHM runtime and its notices (MPL-2.0); PawnIO stays a
# verified download at install time (no stated redistribution permission).
$bundle = [System.IO.Path]::GetFullPath("$PSScriptRoot/../../target/installer-bundle")
if (Test-Path -LiteralPath $bundle) { Remove-Item -LiteralPath $bundle -Recurse -Force }
$null = Get-VerifiedPackage $packages.packages.lhm $bundle
foreach ($notice in $packages.notices) { $null = Get-VerifiedPackage $notice $bundle }
& $Compiler "/DAppVersion=$version" "/DLhmVersion=$($lhmPin.Version)" "/DBundleDir=$bundle" "/DBinDir=$binPath" "/DOutputDir=$outputPath" "$PSScriptRoot/freergb.iss"
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
$installer = Join-Path $outputPath "freeRGB-$version-windows-x86_64-setup.exe"
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText("$installer.sha256", "$hash  $([System.IO.Path]::GetFileName($installer))`n")
Write-Output $installer
