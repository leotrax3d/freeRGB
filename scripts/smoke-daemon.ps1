param([string]$Binary = "$PSScriptRoot/../target/release/freergbd.exe")
$ErrorActionPreference = 'Stop'

if (Get-Process freergbd -ErrorAction SilentlyContinue) {
    throw 'Stop the existing daemon before running this isolated smoke test.'
}
$daemon = Start-Process -FilePath (Resolve-Path -LiteralPath $Binary).Path -ArgumentList '--no-hardware', '--virtual-device', '--no-state' -WindowStyle Hidden -PassThru
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', 'freergb', [System.IO.Pipes.PipeDirection]::InOut)
try {
    $pipe.Connect(10000)
    if ($daemon.HasExited) { throw 'Test daemon exited before connecting.' }
    $reader = [System.IO.StreamReader]::new($pipe)
    $writer = [System.IO.StreamWriter]::new($pipe, [System.Text.UTF8Encoding]::new($false))
    $writer.AutoFlush = $true
    function Invoke-Rpc([string]$Method, $Params = @{}) {
        $writer.WriteLine((@{jsonrpc='2.0'; id=1; method=$Method; params=$Params} | ConvertTo-Json -Compress -Depth 10))
        $read = $reader.ReadLineAsync()
        if (-not $read.Wait(5000)) { throw "Timeout: $Method" }
        $reply = $read.Result | ConvertFrom-Json
        if ($null -eq $reply -or $reply.error) { throw "RPC failed: $Method ($($read.Result))" }
        return $reply.result
    }
    $version = Invoke-Rpc 'daemon.version'
    if ($version.api_version -ne 1) { throw 'Unexpected API version.' }
    $devices = @(Invoke-Rpc 'devices.list')
    if ($devices.Count -ne 1) { throw 'Expected exactly one virtual device.' }
    if (-not (Invoke-Rpc 'color.set' @{color='#123456'})) { throw 'Color update failed.' }
    $effect = Invoke-Rpc 'effects.get'
    if ($effect.type -ne 'solid' -or $effect.color.r -ne 18 -or $effect.color.g -ne 52 -or $effect.color.b -ne 86) {
        throw 'Color round trip failed.'
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $devices = @(Invoke-Rpc 'devices.list')
        if ($devices[0].frames -gt 0) { break }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($devices[0].frames -le 0) { throw 'Virtual device received no frames.' }
    if (-not (Invoke-Rpc 'daemon.shutdown')) { throw 'Shutdown rejected.' }
    if (-not $daemon.WaitForExit(10000)) { throw 'Daemon did not stop.' }
    if ($daemon.ExitCode -ne 0) { throw "Daemon exit code: $($daemon.ExitCode)" }
    Write-Output 'PASS: named-pipe API, virtual rendering, color round trip, clean shutdown.'
} finally {
    $pipe.Dispose()
    if (-not $daemon.HasExited) { Stop-Process -Id $daemon.Id }
    $daemon.Dispose()
}
