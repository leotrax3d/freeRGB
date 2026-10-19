param(
    [string]$Binary = "$PSScriptRoot/../target/release/freergbd.exe",
    [switch]$WithPlugin
)
$ErrorActionPreference = 'Stop'

if (Get-Process freergbd -ErrorAction SilentlyContinue) {
    throw 'Stop the existing daemon before running this isolated smoke test.'
}
$daemon = $null
$manifestPath = $null
$faultManifestPath = $null
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', 'freergb', [System.IO.Pipes.PipeDirection]::InOut)
try {
    $daemonPath = (Resolve-Path -LiteralPath $Binary).Path
    $daemonArgs = @('--no-hardware', '--virtual-device', '--no-state')
    if ($WithPlugin) {
        $pluginPath = (Resolve-Path -LiteralPath (Join-Path (Split-Path $daemonPath) 'freergb-plugin-virtual.exe')).Path
        $manifestPath = Join-Path ([System.IO.Path]::GetTempPath()) ("freergb-smoke-" + [guid]::NewGuid().ToString() + '.json')
        $manifest = @{api_version=1; id='smoke'; name='Plugin smoke'; executable=$pluginPath; access='virtual'; timeout_ms=1000; layout=@{width=1; height=1; leds=@(@{name='LED 1'; pos=@(0,0)})}}
        [System.IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 10), [System.Text.UTF8Encoding]::new($false))
        $daemonArgs += @('--plugin', ('"{0}"' -f $manifestPath))
        $faultManifestPath = $manifestPath + '.fault.json'
        $manifest.id = 'broken'
        $manifest.name = 'Crashing test plugin'
        $manifest.args = @('--fault', 'crash-frame')
        [System.IO.File]::WriteAllText($faultManifestPath, ($manifest | ConvertTo-Json -Depth 10), [System.Text.UTF8Encoding]::new($false))
        $daemonArgs += @('--plugin', ('"{0}"' -f $faultManifestPath))
    }
    $daemon = Start-Process -FilePath $daemonPath -ArgumentList $daemonArgs -WindowStyle Hidden -PassThru
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
    if (@($devices | Where-Object { $_.info.id -eq 'virtual:virtual-strip' }).Count -ne 1) { throw 'Expected the built-in virtual strip.' }
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
    if ($WithPlugin) {
        function Wait-PluginFrames {
            $deadline = [DateTime]::UtcNow.AddSeconds(10)
            do {
                $plugins = @(Invoke-Rpc 'plugins.list' | Where-Object { $_.id -eq 'smoke' })
                if ($plugins.Count -eq 1 -and $plugins[0].frames -gt 0) { return }
                Start-Sleep -Milliseconds 50
            } while ([DateTime]::UtcNow -lt $deadline)
            throw 'Plugin did not receive frames.'
        }
        Wait-PluginFrames
        $deadline = [DateTime]::UtcNow.AddSeconds(10)
        do {
            $broken = @(Invoke-Rpc 'plugins.list' | Where-Object { $_.id -eq 'broken' })
            if ($broken[0].last_error) { break }
            Start-Sleep -Milliseconds 50
        } while ([DateTime]::UtcNow -lt $deadline)
        if (-not $broken[0].last_error) { throw 'Crashing plugin did not report a fault.' }
        if (-not (Invoke-Rpc 'plugins.stop' @{id='broken'})) { throw 'Faulted plugin stop failed.' }
        $before = @(Invoke-Rpc 'plugins.list' | Where-Object { $_.id -eq 'smoke' })[0].frames
        Start-Sleep -Milliseconds 150
        $after = @(Invoke-Rpc 'plugins.list' | Where-Object { $_.id -eq 'smoke' })[0].frames
        if ($after -le $before) { throw 'Healthy plugin stopped rendering after peer crash.' }
        if (-not (Invoke-Rpc 'plugins.stop' @{id='smoke'})) { throw 'Plugin stop failed.' }
        $plugins = @(Invoke-Rpc 'plugins.list' | Where-Object { $_.id -eq 'smoke' })
        if ($plugins[0].enabled -or $null -ne $plugins[0].status) { throw 'Plugin still active after stop.' }
        if (@(Invoke-Rpc 'devices.list').Count -ne 1) { throw 'Plugin device was not removed.' }
        if (-not (Invoke-Rpc 'plugins.start' @{id='smoke'})) { throw 'Plugin start failed.' }
        Wait-PluginFrames
        Write-Output 'PASS: process plugin registration, frames, peer-crash isolation, stop, restart.'
    }
    if (-not (Invoke-Rpc 'daemon.shutdown')) { throw 'Shutdown rejected.' }
    if (-not $daemon.WaitForExit(10000)) { throw 'Daemon did not stop.' }
    if ($daemon.ExitCode -ne 0) { throw "Daemon exit code: $($daemon.ExitCode)" }
    Write-Output 'PASS: named-pipe API, virtual rendering, color round trip, clean shutdown.'
} finally {
    $pipe.Dispose()
    if ($null -ne $daemon) {
        if (-not $daemon.HasExited) { Stop-Process -Id $daemon.Id }
        $daemon.Dispose()
    }
    if ($null -ne $manifestPath -and (Test-Path -LiteralPath $manifestPath)) { Remove-Item -LiteralPath $manifestPath }
    if ($null -ne $faultManifestPath -and (Test-Path -LiteralPath $faultManifestPath)) { Remove-Item -LiteralPath $faultManifestPath }
}
