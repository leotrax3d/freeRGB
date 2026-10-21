param(
    [Parameter(Mandatory=$true)][ValidatePattern('^/lpc/[a-z0-9]+/[0-9]+/control/[0-6]$')][string]$ControlId,
    [switch]$ConsentToProbe,
    [switch]$Elevated,
    [switch]$CrashWorker
)
$ErrorActionPreference='Stop'
if (-not $ConsentToProbe) { throw 'Explicit consent for a temporary100-percent hardware probe is required.' }
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
$admin=([Security.Principal.WindowsPrincipal]$identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    if ($Elevated) { throw 'Elevated probe did not receive administrator rights.' }
    $arguments='-NoProfile -ExecutionPolicy Bypass -File "{0}" -ControlId "{1}" -ConsentToProbe -Elevated' -f $PSCommandPath,$ControlId
    if ($CrashWorker) { $arguments+=' -CrashWorker' }
    $process=Start-Process -FilePath (Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe') -ArgumentList $arguments -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw 'Protected100-percent probe failed or was cancelled; inspect mainboard-probe-result.json.' }
    exit 0
}
$root=Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'freeRGB'
$request=Join-Path $root 'mainboard-probe-request.json'
$result=Join-Path $root 'mainboard-probe-result.json'
$service=Get-CimInstance Win32_Service -Filter "Name='freeRGBMainboard'"
$expected='"'+(Join-Path $root 'MainboardBroker\freergb-mainboard-service.exe')+'"'
if (-not $service -or $service.State -ne 'Running' -or $service.StartName -ne 'LocalSystem' -or $service.PathName -ne $expected) { throw 'Expected protected SYSTEM service is not running.' }
if (Test-Path -LiteralPath $request) { throw 'An existing diagnostic request requires inspection; refusing overwrite.' }
$id=[guid]::NewGuid().ToString('N')
try {
    $payload=@{version=1;id=$id;control_id=$ControlId;expires_utc=[DateTime]::UtcNow.AddSeconds(25).ToString('o')} | ConvertTo-Json -Compress
    # CreateNew refuses preexisting files/reparse targets; root is fixed, service
    # independently verifies protected ownership/ACL before consuming this request.
    $file=[IO.File]::Open($request,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
    try { $bytes=[Text.Encoding]::UTF8.GetBytes($payload);$file.Write($bytes,0,$bytes.Length);$file.Flush($true) } finally { $file.Dispose() }
    $acl=Get-Acl -LiteralPath $request
    $acl.SetOwner([Security.Principal.SecurityIdentifier]::new('S-1-5-32-544'))
    Set-Acl -LiteralPath $request -AclObject $acl
    $controller=New-Object ServiceProcess.ServiceController('freeRGBMainboard')
    try { $controller.ExecuteCommand(128) } finally { $controller.Dispose() }
    $journal=Join-Path $root 'mainboard-control-recovery.json'
    if ($CrashWorker) {
        $armed=$null;$armDeadline=[DateTime]::UtcNow.AddSeconds(6)
        do {
            Start-Sleep -Milliseconds 100
            if(Test-Path -LiteralPath $journal){$armed=(Get-Content -LiteralPath $journal -Raw | ConvertFrom-Json).reservation}
        } while (($null -eq $armed -or $armed.sensor_id -ne $ControlId) -and [DateTime]::UtcNow -lt $armDeadline)
        if ($null -eq $armed -or $armed.sensor_id -ne $ControlId -or -not $armed.recovery.worker_identity) { throw 'No lifetime-bound fixed100 intent was armed; nothing killed.' }
        Start-Sleep -Milliseconds 600
        $still=(Get-Content -LiteralPath $journal -Raw | ConvertFrom-Json).reservation
        if ($null -eq $still -or $still.id -ne $armed.id) { throw 'Probe already returned; nothing killed.' }
        $workerId=[int]($armed.recovery.worker_identity.Split(':')[0])
        $workerPath=Join-Path $root 'MainboardBroker\freergb-mainboard.exe'
        $info=Get-CimInstance Win32_Process -Filter "ProcessId=$workerId"
        $victim=[Diagnostics.Process]::GetProcessById($workerId)
        try {
            $null=$victim.Handle
            $lifetime=$workerId.ToString()+':'+$victim.StartTime.ToUniversalTime().Ticks.ToString()
            if ($info.ExecutablePath -ne $workerPath -or $info.ParentProcessId -ne $service.ProcessId -or $lifetime -ne $armed.recovery.worker_identity) { throw 'Worker lifetime/path/parent mismatch; nothing killed.' }
            $victim.Kill(); if(-not $victim.WaitForExit(3000)){throw 'Owned probe worker could not be reaped.'}
        } finally { $victim.Dispose() }
    }
    $deadline=[DateTime]::UtcNow.AddSeconds(20)
    do {
        Start-Sleep -Milliseconds 200
        if (Test-Path -LiteralPath $result) {
            $reply=Get-Content -LiteralPath $result -Raw | ConvertFrom-Json
            if ($reply.id -eq $id) {
                if ($CrashWorker) {
                    $remaining=(Get-Content -LiteralPath $journal -Raw | ConvertFrom-Json).reservation
                    if ($null -ne $remaining) { continue }
                    $current=Get-CimInstance Win32_Service -Filter "Name='freeRGBMainboard'"
                    if ($current.ProcessId -ne $service.ProcessId -or $reply.result.status -ne 'faulted') { throw 'Crash test did not retain the original supervisor or expected failure result.' }
                    Write-Output 'PASS: fixed100 worker killed; original supervisor restarted it, verified native return, and cleared the recovery intent.'
                    exit 0
                }
                $reply | ConvertTo-Json -Depth 12
                if ($reply.result.status -ne 'completed' -or -not $reply.result.firmware_return_verified -or -not $reply.result.journal_cleared) { throw 'Probe or verified return failed; preserve recovery evidence.' }
                exit 0
            }
        }
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'Probe timed out; inspect protected journal/result before any further control.'
} finally {
    if (Test-Path -LiteralPath $request) {
        $left=Get-Content -LiteralPath $request -Raw | ConvertFrom-Json
        if ($left.id -eq $id) { Remove-Item -LiteralPath $request }
    }
}
