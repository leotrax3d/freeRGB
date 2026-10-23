param([string]$Binary = "$PSScriptRoot/../target/debug/freergbd.exe")
$ErrorActionPreference = 'Stop'
$release = (Resolve-Path -LiteralPath "$PSScriptRoot/../effects/development").Path
$catalog = Get-Content -LiteralPath (Join-Path $release 'catalog.json') -Raw | ConvertFrom-Json
if ($catalog.effects.Count -ne 28) { throw 'Expected 3 examples and 25 collection effects.' }
foreach ($entry in $catalog.effects) {
    $asset = Join-Path $release $entry.artifact.url
    if ((Get-Item -LiteralPath $asset).Length -ne $entry.artifact.size -or (Get-FileHash -LiteralPath $asset -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.artifact.sha256) { throw "Artifact mismatch: $($entry.id)" }
}
$instance = 'effects-' + [guid]::NewGuid().ToString('N')
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', ('freergb-' + $instance), [System.IO.Pipes.PipeDirection]::InOut, [System.IO.Pipes.PipeOptions]::Asynchronous)
$daemon = $null
$log = Join-Path ([System.IO.Path]::GetTempPath()) ($instance + '.log')
$errorsLog = $log + '.err'
try {
    $daemonPath = (Resolve-Path -LiteralPath $Binary).Path
    $daemonArgs = @('--no-hardware','--no-modules','--virtual-device','--no-state','--ipc-instance',$instance,'--effect-dev-directory',('"{0}"' -f $release))
    $daemon = Start-Process -FilePath $daemonPath -ArgumentList $daemonArgs -WindowStyle Hidden -PassThru -RedirectStandardOutput $log -RedirectStandardError $errorsLog
    $pipe.Connect(15000)
    $reader = [System.IO.StreamReader]::new($pipe)
    $writer = [System.IO.StreamWriter]::new($pipe,[System.Text.UTF8Encoding]::new($false))
    $writer.AutoFlush = $true
    function Invoke-EffectRpc([string]$Method, $Params = @{}) {
        Write-Verbose $Method
        $writer.WriteLine((@{jsonrpc='2.0';id=1;method=$Method;params=$Params} | ConvertTo-Json -Compress -Depth 15))
        $read = $reader.ReadLineAsync()
        if (-not $read.Wait(10000)) { throw "Timeout: $Method" }
        $reply = $read.Result | ConvertFrom-Json
        if ($null -eq $reply -or $reply.error) { throw "RPC failed: $Method ($($read.Result))" }
        return $reply.result
    }
    function Wait-EffectJob {
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $status = Invoke-EffectRpc 'marketplace.status'
            if (-not $status.packages.job) { break }
            Start-Sleep -Milliseconds 20
        } while ([DateTime]::UtcNow -lt $deadline)
        if ($status.packages.job -or $status.packages.error) { throw "Marketplace job failed: $($status.packages.error)" }
        if ($status.packages.ratings.configured) { throw 'Development source unexpectedly connected to live ratings.' }
        return $status
    }
    Invoke-EffectRpc 'marketplace.refresh' | Out-Null
    $status = Wait-EffectJob
    if ($status.packages.catalog.Count -ne 28) { throw 'Development catalog not loaded.' }
    foreach ($entry in ($catalog.effects | Select-Object -Skip 3)) {
        Invoke-EffectRpc 'marketplace.install' @{id=$entry.id} | Out-Null
        Wait-EffectJob | Out-Null
        foreach ($maximum in @($false,$true)) {
            $parameters = @{}
            foreach ($p in $entry.parameters) { $parameters[$p.key] = $(if ($maximum) {$p.max} else {$p.default}) }
            Invoke-EffectRpc 'effects.set' @{type='marketplace';id=$entry.id;version=$entry.version;parameters=$parameters} | Out-Null
            Start-Sleep -Milliseconds 250
            $status = Invoke-EffectRpc 'marketplace.status'
            if ($status.runtime_error -or $status.active.id -ne $entry.id) { throw "Activation/render failed for $($entry.id): $($status.runtime_error)" }
        }
        Invoke-EffectRpc 'effects.set' @{type='solid';color=@{r=0;g=0;b=0}} | Out-Null
        Invoke-EffectRpc 'marketplace.uninstall' @{id=$entry.id} | Out-Null
        Write-Host "Passed: $($entry.name)"
    }
    $devices = @(Invoke-EffectRpc 'devices.list')
    if (-not ($devices | Where-Object {$_.frames -gt 0})) { throw 'Virtual device received no frames.' }
    Write-Host '25 effects passed isolated Windows IPC install/render/configure/uninstall smoke.'
} catch {
    if (Test-Path -LiteralPath $errorsLog) { Get-Content -LiteralPath $errorsLog | Write-Host }
    throw
} finally {
    $pipe.Dispose()
    if ($daemon -and -not $daemon.HasExited) { Stop-Process -Id $daemon.Id; $daemon.WaitForExit() }
}
