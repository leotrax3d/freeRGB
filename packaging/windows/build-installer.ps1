param([string]$Compiler, [string]$BinDir = "$PSScriptRoot/../../target/release", [string]$OutputDir = "$PSScriptRoot/../../target/dist")
$ErrorActionPreference = 'Stop'
if (-not $Compiler) {
    $candidates = @("${env:ProgramFiles(x86)}/Inno Setup 6/ISCC.exe", "$env:ProgramFiles/Inno Setup 7/ISCC.exe", "$env:ProgramFiles/Inno Setup 6/ISCC.exe")
    $Compiler = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $Compiler) { throw 'Inno Setup compiler not found. Pass -Compiler PATH or build using the GitHub Package workflow.' }
$workspace = [IO.Path]::GetFullPath("$PSScriptRoot/../..")
$metadata = cargo metadata --manifest-path "$workspace/Cargo.toml" --format-version 1 --no-deps --locked | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$version = ($metadata.packages | Where-Object { $_.name -eq 'freergb-gui' }).version
if ($env:GITHUB_REF_TYPE -eq 'tag' -and $env:GITHUB_REF_NAME -ne "v$version") { throw 'Release tag must match the Cargo workspace version.' }
$outputPath = [System.IO.Path]::GetFullPath($OutputDir)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
if (Get-ChildItem -LiteralPath $outputPath -Force) { throw 'Installer output directory must be empty; use a new OutputDir to avoid uploading stale artifacts.' }
$binPath = (Resolve-Path -LiteralPath $BinDir).Path
& "$PSScriptRoot/../../scripts/build-desktop.ps1" -OutputDirectory $binPath
& "$PSScriptRoot/../../scripts/build-mainboard-helper.ps1" -OutputDirectory $binPath -Mode Production
& "$PSScriptRoot/../../scripts/build-keyboard-helper.ps1" -OutputDirectory $binPath
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
$payload = [ordered]@{}
foreach ($file in @('freergb.exe','freergbd.exe','freergb-launcher.exe','freergb-gui.exe','freergb-desktop.exe','freergb-tray.exe','freergb-mainboard.exe','freergb-mainboard-service.exe','freergb-keyboard-input.exe')) {
    $payload[$file] = (Get-FileHash -LiteralPath (Join-Path $binPath $file) -Algorithm SHA256).Hash.ToLowerInvariant()
}
$commit = (& git -C $workspace rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot determine build commit.' }
$changes = @(& git -C $workspace status --porcelain --untracked-files=normal)
if ($LASTEXITCODE -ne 0) { throw 'Cannot determine source state.' }
$dirty = $changes.Count -ne 0
$manifest = [ordered]@{version=$version;commit=$commit;source_dirty=$dirty;
    rustc=(& rustc --version);node=(& node --version);
    compiler_version=(Get-Item -LiteralPath $Compiler).VersionInfo.FileVersion;
    compiler_sha256=(Get-FileHash -LiteralPath $Compiler -Algorithm SHA256).Hash.ToLowerInvariant();
    authenticode=(Get-AuthenticodeSignature -LiteralPath $installer).Status.ToString();
    installer=@{file=[IO.Path]::GetFileName($installer);size=(Get-Item -LiteralPath $installer).Length;sha256=$hash};payload=$payload}
[IO.File]::WriteAllText((Join-Path $outputPath "freeRGB-$version-windows-x86_64-build.json"),($manifest | ConvertTo-Json -Depth 5) + "`n")
Write-Output $installer
