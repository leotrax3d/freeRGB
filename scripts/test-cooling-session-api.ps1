param([string]$Daemon = "$PSScriptRoot/../target/debug/freergbd.exe")
$ErrorActionPreference = 'Stop'
$binary = (Resolve-Path -LiteralPath $Daemon).Path
$instance = 'cooling-api-' + [Guid]::NewGuid().ToString('N')
$process = Start-Process -FilePath $binary -ArgumentList @('--no-hardware','--no-state','--ipc-instance',$instance) -WindowStyle Hidden -PassThru
$pipe = [IO.Pipes.NamedPipeClientStream]::new('.',('freergb-' + $instance),[IO.Pipes.PipeDirection]::InOut)
try {
    $pipe.Connect(10000)
    $reader = [IO.StreamReader]::new($pipe)
    $writer = [IO.StreamWriter]::new($pipe)
    $writer.AutoFlush = $true
    function Call-Rpc([string]$method, $parameters) {
        $writer.WriteLine((@{jsonrpc='2.0';id=1;method=$method;params=$parameters} | ConvertTo-Json -Compress -Depth 10))
        $reply = $reader.ReadLineAsync()
        if (-not $reply.Wait(5000)) { throw 'Isolated daemon RPC timed out.' }
        return ($reply.Result | ConvertFrom-Json)
    }
    $status = Call-Rpc 'cooling.sessions.status' @{}
    if ($status.result.available -or $status.result.state -ne 'disabled') { throw 'No-hardware daemon enabled control.' }
    $request = @{mode='calibration';control_id='/lpc/nct6701d/0/control/2';rpm_id='/lpc/nct6701d/0/fan/2';temperature_id='/lpc/nct6701d/0/temperature/1';confirmed_fan=$true}
    if ((Call-Rpc 'cooling.sessions.start' $request).error.code -ne -32006) { throw 'Disabled start accepted.' }
    if ((Call-Rpc 'cooling.sessions.start' @{}).error.code -ne -32602) { throw 'Malformed start accepted.' }
    $request.extra = 1
    if ((Call-Rpc 'cooling.sessions.start' $request).error.code -ne -32602) { throw 'Unknown start parameter accepted.' }
    if ((Call-Rpc 'cooling.sessions.stop' @{job_id=1}).error.code -ne -32006) { throw 'Inactive stop accepted.' }
    if ((Call-Rpc 'cooling.sessions.stop' @{}).error.code -ne -32602) { throw 'Malformed stop accepted.' }
    $stop = Call-Rpc 'daemon.shutdown_if' @{expected_executable=$binary}
    if (-not $stop.result -or -not $process.WaitForExit(10000)) { throw 'Owned daemon failed to exit.' }
    Write-Output 'PASS: isolated real daemon pipe, disabled status/start/stop, malformed/unknown fields and owned shutdown. No hardware or UAC.'
} finally {
    $pipe.Dispose()
    if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
    $process.Dispose()
}
