# Disposable hosted-runner acceptance only. NEVER register a service on a developer PC.
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or -not $env:RUNNER_TEMP -or $env:RUNNER_OS -ne 'Windows') {
    throw 'SCM acceptance is restricted to disposable GitHub Windows runners.'
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
if (-not ([Security.Principal.WindowsPrincipal]$identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Disposable runner must have an elevated token.'
}
$protected = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'freeRGB\MainboardBroker'
$serviceKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\freeRGBMainboard'
if ((Test-Path -LiteralPath $protected) -or (Test-Path -LiteralPath $serviceKey)) { throw 'Refusing to touch an existing broker installation.' }
$root = [IO.Path]::GetFullPath("$PSScriptRoot/../target/mainboard-service-acceptance")
New-Item -ItemType Directory -Path $root -Force | Out-Null
$username = 'frgb' + [guid]::NewGuid().ToString('N').Substring(0,12)
$password = ConvertTo-SecureString ([guid]::NewGuid().ToString('N') + '!aA9') -AsPlainText -Force
$user = New-LocalUser -Name $username -Password $password -AccountNeverExpires
$credential = [pscredential]::new("$env:COMPUTERNAME\$username", $password)
$sid = $user.SID.Value
$userGroup = Get-LocalGroup -SID 'S-1-5-32-545'
Add-LocalGroupMember -Group $userGroup.Name -Member $username
$acl = Get-Acl -LiteralPath $root
$acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($user.SID,'Modify','ContainerInherit,ObjectInherit','None','Allow'))
Set-Acl -LiteralPath $root -AclObject $acl
$v1 = Join-Path $root 'v1'; $v2 = Join-Path $root 'v2'; $bad = Join-Path $root 'bad'
function Require($value,[string]$message) { if (-not $value) { throw $message } }
function Maintain([string]$binary,[bool]$success,[string[]]$arguments) {
    & $binary @arguments
    $code = $LASTEXITCODE
    Require (($code -eq 0) -eq $success) "Unexpected maintenance exit: $code"
}
function Install([string]$directory,[bool]$success) {
    $binary = Join-Path $directory 'freergb-mainboard-service.exe'
    $hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
    Maintain $binary $success @('--install','--consent',$sid,$hash,$archive,$cache)
}
function Snapshot([int]$expected,[pscredential]$As = $credential,[switch]$ExpectDenied) {
    $request = Join-Path $root 'request.txt'; [IO.File]::WriteAllText($request,"snapshot`n")
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    do {
        $output = Join-Path $root ('reading-' + [guid]::NewGuid().ToString('N') + '.json')
        $errorLog = "$output.err"
        $process = Start-Process -FilePath (Join-Path $v1 'freergb-mainboard.exe') -ArgumentList @('--library',('"{0}"' -f $root)) -WorkingDirectory $root -Credential $As -WindowStyle Hidden -Wait -PassThru -RedirectStandardInput $request -RedirectStandardOutput $output -RedirectStandardError $errorLog
        Require ($process.ExitCode -eq 0) 'Standard-user reader failed.'
        $reply = Get-Content -LiteralPath $output -Raw | ConvertFrom-Json
        if ($ExpectDenied) { Require ($reply.status -eq 'faulted' -and $reply.hardware.Count -eq 0) 'Unauthorized account read the broker.'; return }
        if ($reply.status -eq 'ready' -and $reply.hardware[0].sensors[0].value -eq $expected -and $reply.message -eq 'service_client_authenticated') { return }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Standard-user broker did not return expected fixture value $expected. Last reply: $($reply | ConvertTo-Json -Depth 8 -Compress)"
}
$foreignName = $null
try {
    Import-Module "$PSScriptRoot/../packaging/windows/mainboard-prerequisites.psm1" -Force
    $manifest = Read-MainboardPackages "$PSScriptRoot/../packaging/windows/mainboard-packages.json"
    $cache = Join-Path $root 'downloads'
    $archive = Get-VerifiedPackage $manifest.packages.lhm $cache
    foreach ($notice in $manifest.notices) { $null = Get-VerifiedPackage $notice $cache }
    & "$PSScriptRoot/build-mainboard-helper.ps1" -OutputDirectory $v1 -Mode FixtureV1
    & "$PSScriptRoot/build-mainboard-helper.ps1" -OutputDirectory $v2 -Mode FixtureV2
    & "$PSScriptRoot/build-mainboard-helper.ps1" -OutputDirectory $bad -Mode FixtureFailStart
    Install $v1 $true; Snapshot 900
    $foreignName = 'frgb' + [guid]::NewGuid().ToString('N').Substring(0,12)
    $foreignPassword = ConvertTo-SecureString ([guid]::NewGuid().ToString('N') + '!aA9') -AsPlainText -Force
    $foreign = New-LocalUser -Name $foreignName -Password $foreignPassword -AccountNeverExpires
    Add-LocalGroupMember -Group $userGroup.Name -Member $foreignName
    $acl = Get-Acl -LiteralPath $root
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($foreign.SID,'Modify','ContainerInherit,ObjectInherit','None','Allow'))
    Set-Acl -LiteralPath $root -AclObject $acl
    Snapshot 0 -As ([pscredential]::new("$env:COMPUTERNAME\$foreignName",$foreignPassword)) -ExpectDenied
    $before = (Get-FileHash -LiteralPath (Join-Path $protected 'freergb-mainboard-service.exe')).Hash
    Install $bad $false; Snapshot 900
    Require ((Get-FileHash -LiteralPath (Join-Path $protected 'freergb-mainboard-service.exe')).Hash -eq $before) 'Failed start did not restore old executable bytes.'
    Install $v2 $true; Snapshot 901
    # Kill only the exact owned synthetic child; SCM and the snapshot pipe must survive.
    $servicePid = (Get-CimInstance Win32_Service -Filter "Name='freeRGBMainboard'").ProcessId
    $workerPath = Join-Path $protected 'freergb-mainboard.exe'
    $children = @(Get-CimInstance Win32_Process -Filter "Name='freergb-mainboard.exe'" | Where-Object { $_.ExecutablePath -eq $workerPath -and $_.ParentProcessId -eq $servicePid })
    Require ($children.Count -eq 1) 'Expected exactly one owned synthetic worker.'
    $oldWorker = $children[0].ProcessId
    Stop-Process -Id $oldWorker -Force
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    do {
        Start-Sleep -Milliseconds 250
        $children = @(Get-CimInstance Win32_Process -Filter "Name='freergb-mainboard.exe'" | Where-Object { $_.ExecutablePath -eq $workerPath -and $_.ParentProcessId -eq $servicePid -and $_.ProcessId -ne $oldWorker })
    } while ($children.Count -ne 1 -and [DateTime]::UtcNow -lt $deadline)
    Require ($children.Count -eq 1) 'Supervisor did not replace its crashed synthetic worker.'
    Require ((Get-CimInstance Win32_Service -Filter "Name='freeRGBMainboard'").ProcessId -eq $servicePid) 'Worker crash unexpectedly restarted the service.'
    Snapshot 901
    # A changed user-writable helper must be rejected before the running service stops.
    [IO.File]::AppendAllText((Join-Path $bad 'freergb-mainboard.exe'),'tampered')
    Install $bad $false; Snapshot 901
    Maintain (Join-Path $v2 'freergb-mainboard-service.exe') $false @('--remove','--consent',$foreign.SID.Value)
    Snapshot 901
    Maintain (Join-Path $v2 'freergb-mainboard-service.exe') $true @('--remove','--consent',$sid)
    Require (-not (Test-Path -LiteralPath $protected)) 'Protected owned files were not removed.'
    Require (-not (Test-Path -LiteralPath $serviceKey)) 'Service registration was not removed.'
    Write-Output 'PASS: real SCM install, standard-user read, unauthorized-account denial, failed-version rollback, update, isolated child crash recovery, tamper/owner rejection and owned removal. Compile-time sensor fixture; no LHM driver access.'
} finally {
    # Cleanup is restricted to the initially absent, explicitly owned fixture service.
    if ((Test-Path -LiteralPath $serviceKey) -and (Test-Path -LiteralPath (Join-Path $v2 'freergb-mainboard-service.exe'))) {
        & (Join-Path $v2 'freergb-mainboard-service.exe') --remove --consent $sid
    }
    Remove-LocalUser -Name $username -ErrorAction SilentlyContinue
    if ($foreignName) { Remove-LocalUser -Name $foreignName -ErrorAction SilentlyContinue }
}
