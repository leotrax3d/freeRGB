param(
    [string]$Binary = "$PSScriptRoot/../target/release/freergbd.exe",
    [switch]$WithPlugin
)
$ErrorActionPreference = 'Stop'

$instance = 'smoke-' + [guid]::NewGuid().ToString('N')
$daemon = $null
$manifestPath = $null
$faultManifestPath = $null
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', ('freergb-' + $instance), [System.IO.Pipes.PipeDirection]::InOut)
try {
    $daemonPath = (Resolve-Path -LiteralPath $Binary).Path
    $daemonArgs = @('--no-hardware', '--virtual-device', '--no-state','--ipc-instance',$instance)
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
    if (Invoke-Rpc 'daemon.shutdown_if' @{expected_executable=[System.Environment]::ProcessPath}) { throw 'Guard stopped a daemon from a different executable.' }
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
    $curve = @{points=@(@{temperature_c=25; duty_percent=30},@{temperature_c=75; duty_percent=100}); minimum_percent=25; critical_c=85; hysteresis_c=2}
    $calibration = Invoke-Rpc 'cooling.calibration.preview' @{role='fan';critical_c=85;samples=@(@{at_ms=0;applied_sequence=1;rpm=0;rpm_age_ms=0;temperature_c=40;temperature_age_ms=0})}
    if (-not $calibration.simulated -or $calibration.hardware_control -or $calibration.final_output.phase -ne 'restoring' -or $calibration.final_output.duty_percent -ne 100) { throw 'Calibration policy failed to request simulated stall recovery.' }
    $sensorGraph = @{version=1;nodes=@{input=@{type='input';source='explicit-unavailable-test-source'};adjusted=@{type='offset';source='input';amount=2;proportional=$false}}}
    if (-not (Invoke-Rpc 'cooling.sensors.save' @{graph=$sensorGraph})) { throw 'Custom sensor save failed.' }
    $sensorValues = Invoke-Rpc 'cooling.sensors.values'
    if ($null -ne $sensorValues.values.adjusted.value) { throw 'Missing source became a fabricated reading.' }
    $storedGraph = Invoke-Rpc 'cooling.sensors.get'
    $providers = @(Invoke-Rpc 'cooling.providers.list')
    if ($providers.Count -ne 1 -or $providers[0].status -ne 'disabled' -or $providers[0].control_enabled) { throw 'Optional motherboard provider unexpectedly enabled.' }
    if (@(Invoke-Rpc 'cooling.channels.list').Count -ne 0) { throw 'Disabled provider fabricated channels.' }
    $bindings = Invoke-Rpc 'cooling.bindings.get'
    if ($bindings.config.version -ne 1 -or $bindings.persistence_enabled -or @($bindings.config.bindings).Count -ne 0) { throw 'Unexpected default channel bindings.' }
    $bindings.config.bindings = @(@{provider='mainboard.lhm';device_id='offline-board';control_id='offline-control';tachometer_id=$null;header='CPU_FAN';role='fan';electrical_mode='unknown'})
    if (-not (Invoke-Rpc 'cooling.bindings.save' @{config=$bindings.config})) { throw 'Offline channel annotation failed.' }
    $bindings = Invoke-Rpc 'cooling.bindings.get'
    if ($bindings.assignments[0].state -ne 'unavailable' -or $bindings.assignments[0].hardware_verified -or $bindings.assignments[0].writable) { throw 'Annotation implied hardware verification or write permission.' }
    if (-not (Invoke-Rpc 'cooling.bindings.save' @{config=@{version=1;bindings=@()}})) { throw 'Clearing channel annotations failed.' }
    $null = Invoke-Rpc 'monitoring.sensors.list'
    $keyboard = Invoke-Rpc 'keyboard.reactive.get'
    if ($keyboard.config.enabled -or $keyboard.status.records_text) { throw 'Keyboard capture should default off without recording text.' }
    $keyboard.config.enabled = $true
    $keyboard.config.devices = @('virtual:virtual-strip')
    if (-not (Invoke-Rpc 'keyboard.reactive.configure' @{config=$keyboard.config})) { throw 'Keyboard configuration failed.' }
    if ((Invoke-Rpc 'keyboard.reactive.get').status.state -ne 'blocked_no_hardware') { throw 'No-hardware mode allowed input capture.' }
    if (-not (Invoke-Rpc 'keyboard.reactive.preview' @{usage=4})) { throw 'Keyboard preview enqueue failed.' }
    $disabled = Invoke-Rpc 'keyboard.reactive.disable'
    if (-not $disabled.stopped -or $disabled.persisted) { throw 'No-state keyboard stop failed.' }
    if ($storedGraph.nodes.adjusted.amount -ne 2 -or $storedGraph.nodes.input.source -ne 'explicit-unavailable-test-source') { throw 'Custom sensor round trip failed.' }
    if (-not (Invoke-Rpc 'cooling.curves.save' @{name='Smoke'; curve=$curve})) { throw 'Curve save failed.' }
    $preview = Invoke-Rpc 'cooling.preview' @{curve=$curve; temperature_c=50; age_ms=0}
    if ($preview.duty_percent -ne 65 -or -not $preview.simulated) { throw 'Cooling interpolation failed.' }
    $stale = Invoke-Rpc 'cooling.preview' @{curve=$curve; temperature_c=50; age_ms=3001}
    if ($stale.duty_percent -ne 100) { throw 'Cooling fail-safe failed.' }
    $definition = @{repeat=1; steps=@(@{type='delay'; ms=50},@{type='color'; color=@{r=1;g=2;b=3}})}
    if (-not (Invoke-Rpc 'macros.save' @{name='Smoke'; definition=$definition})) { throw 'Macro save failed.' }
    if (-not (Invoke-Rpc 'macros.run' @{name='Smoke'})) { throw 'Macro run failed.' }
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $effect = Invoke-Rpc 'effects.get'
        if ($effect.type -eq 'solid' -and $effect.color.r -eq 1 -and $effect.color.g -eq 2 -and $effect.color.b -eq 3) { break }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($effect.type -ne 'solid' -or $effect.color.r -ne 1 -or $effect.color.g -ne 2 -or $effect.color.b -ne 3) { throw 'Macro did not run to completion.' }
    Invoke-Rpc 'macros.cancel' | Out-Null
    Write-Output 'PASS: cooling curves, stale-sensor fail-safe, RGB macro scheduling.'
    $report = Invoke-Rpc 'diagnostics.export'
    if ($report.report_version -ne 1 -or @($report.devices).Count -lt 1) { throw 'Diagnostic export failed.' }
    if ($null -ne $report.devices[0].serial -or $null -ne $report.devices[0].id) { throw 'Diagnostic export contains raw device identifiers.' }
    Write-Output 'PASS: redacted diagnostic export.'
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
    if (-not (Invoke-Rpc 'daemon.shutdown_if' @{expected_executable=$daemonPath})) { throw 'Guarded shutdown rejected.' }
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
