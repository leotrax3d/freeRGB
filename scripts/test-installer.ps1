param(
    [string]$Compiler,
    [string]$Installer = "$PSScriptRoot/../target/dist/freeRGB-0.1.0-windows-x86_64-setup.exe",
    [string]$BinDir = "$PSScriptRoot/../target/release",
    [switch]$ModelOnly
)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Windows is required.' }
if (-not $ModelOnly -and ($env:GITHUB_ACTIONS -ne 'true' -or -not $env:RUNNER_TEMP)) {
    throw 'Installation lifecycle tests run only on an ephemeral GitHub Windows runner. Use -ModelOnly locally.'
}
if (-not $Compiler) {
    $Compiler = @("${env:ProgramFiles(x86)}/Inno Setup 6/ISCC.exe", "$env:ProgramFiles/Inno Setup 7/ISCC.exe", "$env:ProgramFiles/Inno Setup 6/ISCC.exe") |
        Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $Compiler) { throw 'Pass -Compiler with an Inno Setup compiler.' }
function Assert-True($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Invoke-TestProcess([string]$File, [string[]]$Arguments, [int[]]$Allowed = @(0), [int]$Timeout = 60000) {
    $process = Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit($Timeout)) {
        Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
        throw "Timed out: $File"
    }
    Assert-True ($Allowed -contains $process.ExitCode) "Unexpected exit $($process.ExitCode): $File"
}
$workspace = (Resolve-Path "$PSScriptRoot/..").Path
$logs = Join-Path $workspace 'target/installer-acceptance'
New-Item -ItemType Directory -Path $logs -Force | Out-Null
& $Compiler "$workspace/packaging/windows/installer-tests.iss" *> "$logs/model-compile.log"
if ($LASTEXITCODE -ne 0) { throw 'Installer model harness compilation failed.' }
$marker = Join-Path $logs 'model-result.txt'
if (Test-Path -LiteralPath $marker) { Remove-Item -LiteralPath $marker }
Invoke-TestProcess "$workspace/target/installer-tests/installer-model-tests.exe" @('/VERYSILENT','/SUPPRESSMSGBOXES',('/RESULT="{0}"' -f $marker)) @(1)
Assert-True ((Test-Path -LiteralPath $marker) -and (Get-Content -LiteralPath $marker -Raw).StartsWith('PASS:')) 'Installer model assertions failed.'
Get-Content -LiteralPath $marker
if ($ModelOnly) { exit 0 }

# Never use this acceptance script on an existing personal installation.
$uninstallKey = 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Uninstall/{A635E27A-ADBF-4DC2-8BF4-4237EC0C96ED}_is1'
$runKey = 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Run'
$config = Join-Path $env:APPDATA 'freeRGB'
Assert-True (-not (Test-Path -LiteralPath $uninstallKey)) 'A freeRGB installation already exists.'
Assert-True (-not (Test-Path -LiteralPath $config)) 'Existing freeRGB user data must not be touched.'
Assert-True (-not (Get-Process freergbd -ErrorAction SilentlyContinue)) 'A daemon already exists.'
Assert-True ($null -eq (Get-ItemProperty -LiteralPath $runKey -Name freeRGB -ErrorAction SilentlyContinue)) 'Existing startup registration must not be touched.'
$root = Join-Path ([System.IO.Path]::GetFullPath($env:RUNNER_TEMP)) ('freeRGB Installer ' + [guid]::NewGuid().ToString('N'))
$app = Join-Path $root 'installed app'
$groupName = 'freeRGB'
$group = Join-Path ([Environment]::GetFolderPath('Programs')) $groupName
$desktop = Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) 'freeRGB.lnk'
Assert-True (-not (Test-Path -LiteralPath $desktop)) 'An existing desktop shortcut must not be touched.'
Assert-True (-not (Test-Path -LiteralPath $group)) 'An existing Start Menu group must not be touched.'
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$BinDir = (Resolve-Path -LiteralPath $BinDir).Path
New-Item -ItemType Directory -Path "$config/profiles" -Force | Out-Null
$sentinel = Join-Path $config 'profiles/InstallerSentinel.json'
[System.IO.File]::WriteAllText($sentinel, 'preserve user data')
function Install-Test([string]$Name, [string]$Tasks) {
    $arguments = @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/DIR="{0}"' -f $app),('/LOG="{0}"' -f (Join-Path $logs "$Name.log")))
    if ($Tasks -ne 'DEFAULT') { $arguments += "/TASKS=$Tasks" }
    Invoke-TestProcess $Installer $arguments
    Assert-True ((Get-Content -LiteralPath $sentinel -Raw) -eq 'preserve user data') 'Setup changed user profiles.'
}
function Get-Startup { (Get-ItemProperty -LiteralPath $runKey -Name freeRGB -ErrorAction SilentlyContinue).freeRGB }
function Start-Virtual([string]$Directory) {
    $process = Start-Process -FilePath (Join-Path $Directory 'freergbd.exe') -ArgumentList @('--no-hardware','--virtual-device','--no-state') -WindowStyle Hidden -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        & (Join-Path $Directory 'freergb.exe') ctl daemon.version *> "$logs/daemon-ready.log"
        if ($LASTEXITCODE -eq 0) { return $process }
        Assert-True (-not $process.HasExited) 'Virtual daemon exited during startup.'
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
    throw 'Virtual daemon did not become ready.'
}
$daemon = $null
$fixtureBrokerInstalled = $false
try {
    Install-Test 'fresh-defaults' 'DEFAULT'
    foreach ($file in @('freergb.exe','freergbd.exe','freergb-gui.exe','freergb-launcher.exe','freergb-mainboard.exe','freergb-mainboard-service.exe','unins000.exe')) {
        Assert-True (Test-Path -LiteralPath (Join-Path $app $file)) "Missing installed file $file"
    }
    Assert-True (Test-Path -LiteralPath "$group/freeRGB.lnk") 'Start Menu should default on.'
    Assert-True (-not (Test-Path -LiteralPath $desktop)) 'Desktop should default off.'
    Assert-True ((Get-Startup) -eq ('"' + (Join-Path $app 'freergb-launcher.exe') + '"')) 'Autostart should default on and quote spaces.'
    foreach ($file in @('install-mainboard.ps1','remove-mainboard.ps1','mainboard-prerequisites.psm1','mainboard-packages.json')) {
        Assert-True (Test-Path -LiteralPath (Join-Path $app $file)) "Missing optional prerequisite bootstrap file $file"
    }
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $app 'mainboard-provider.json'))) 'Default/silent setup must not enable optional motherboard access.'
    Assert-True (-not (Test-Path -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard')) 'Default/silent setup must not create a privileged service.'
    Assert-True (Test-Path -LiteralPath $uninstallKey) 'Per-user uninstall registration missing.'
    $daemon = Start-Virtual $app
    Install-Test 'upgrade-options-off' 'desktopicon'
    Assert-True ($daemon.WaitForExit(10000)) 'Upgrade did not gracefully stop its installed daemon.'
    Assert-True ($daemon.ExitCode -eq 0) 'Installed daemon did not exit cleanly.'
    $daemon = $null
    Assert-True (-not (Test-Path -LiteralPath "$group/freeRGB.lnk")) 'Deselected Start Menu shortcut remains.'
    Assert-True (Test-Path -LiteralPath $desktop) 'Selected desktop shortcut missing.'
    Assert-True ($null -eq (Get-Startup)) 'Deselected startup entry remains.'
    $daemon = Start-Virtual $BinDir
    Install-Test 'upgrade-other-copy-running' 'startmenu,autostart'
    Assert-True (-not $daemon.HasExited) 'Setup stopped a daemon from another checkout.'
    & (Join-Path $BinDir 'freergb.exe') stop-installed-daemon --directory $BinDir
    Assert-True ($LASTEXITCODE -eq 0 -and $daemon.WaitForExit(10000)) 'Unrelated test daemon cleanup failed.'
    $daemon = $null
    Assert-True (-not (Test-Path -LiteralPath $desktop)) 'Deselected desktop shortcut remains.'
    # Real uninstall hook with an owned synthetic service; never load hardware/driver code.
    Assert-True (-not (Test-Path -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard')) 'An unrelated broker exists before installer acceptance.'
    $fixtureDirectory = Join-Path $root 'synthetic broker maintenance'
    & "$PSScriptRoot/build-mainboard-helper.ps1" -OutputDirectory $fixtureDirectory -Mode FixtureV1
    $maintenance = Join-Path $fixtureDirectory 'freergb-mainboard-service.exe'
    $manifest = Get-Content -LiteralPath "$workspace/packaging/windows/mainboard-packages.json" -Raw | ConvertFrom-Json
    $cache = Join-Path $workspace 'target/prerequisite-tests/downloads'
    $archive = Join-Path $cache $manifest.packages.lhm.file
    $ownerSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $hash = (Get-FileHash -LiteralPath $maintenance).Hash
    & $maintenance --install --consent $ownerSid $hash $archive $cache
    Assert-True ($LASTEXITCODE -eq 0) 'Owned synthetic broker installation failed.'
    $fixtureBrokerInstalled = $true
    Assert-True (Test-Path -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard') 'Synthetic broker not registered.'
    $daemon = Start-Virtual $app
    Invoke-TestProcess "$app/unins000.exe" @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="{0}"' -f (Join-Path $logs 'uninstall.log')))
    Assert-True ($daemon.WaitForExit(10000) -and $daemon.ExitCode -eq 0) 'Uninstall did not gracefully stop its daemon.'
    $daemon = $null
    foreach ($file in @('freergb.exe','freergbd.exe','freergb-gui.exe','freergb-launcher.exe','freergb-mainboard.exe','freergb-mainboard-service.exe','remove-mainboard.ps1')) {
        Assert-True (-not (Test-Path -LiteralPath (Join-Path $app $file))) "Installed file remains: $file"
    }
    Assert-True (-not (Test-Path -LiteralPath $uninstallKey)) 'Uninstall registry entry remains.'
    Assert-True (-not (Test-Path -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard')) 'Uninstall did not remove its owned broker registration.'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'freeRGB/MainboardBroker'))) 'Uninstall did not remove owned protected broker files.'
    Assert-True ($null -eq (Get-Startup)) 'Startup entry remains after uninstall.'
    Assert-True (-not (Test-Path -LiteralPath "$group/freeRGB.lnk")) 'Start Menu shortcut remains after uninstall.'
    Assert-True ((Get-Content -LiteralPath $sentinel -Raw) -eq 'preserve user data') 'Uninstall deleted profiles.'
    'PASS: per-user install, default tasks, reinstall/upgrade path, option removal, scoped shutdown, uninstall and preserved profiles.' | Tee-Object -FilePath "$logs/lifecycle-result.txt"
} finally {
    if ($null -ne $daemon -and -not $daemon.HasExited) { Stop-Process -Id $daemon.Id -ErrorAction SilentlyContinue }
    if ($fixtureBrokerInstalled -and (Test-Path -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard')) {
        & $maintenance --remove --consent $ownerSid
    }
}
