Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Read-MainboardPackages([string]$Path) {
    $manifest = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    if ($manifest.version -ne 1) { throw 'Unsupported prerequisite manifest version.' }
    foreach ($package in @($manifest.packages.lhm,$manifest.packages.pawnio,$manifest.packages.dotnet) + @($manifest.notices)) {
        $uri = [uri]$package.url
        if ($uri.Scheme -ne 'https' -or $uri.Host -notin @('github.com','raw.githubusercontent.com','go.microsoft.com') -or
            $package.sha256 -notmatch '^[a-f0-9]{64}$' -or $package.file -notmatch '^[A-Za-z0-9_.-]+$' -or
            $package.size -le 0 -or $package.size -gt 150MB) { throw 'Invalid prerequisite download entry.' }
    }
    return $manifest
}

function Test-PackageHash([string]$Path, $Package) {
    return (Test-Path -LiteralPath $Path -PathType Leaf) -and ((Get-Item -LiteralPath $Path).Length -eq $Package.size) -and
        ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -eq $Package.sha256)
}

function Get-VerifiedPackage($Package, [string]$Cache) {
    New-Item -ItemType Directory -Path $Cache -Force | Out-Null
    $destination = Join-Path $Cache $Package.file
    if (-not (Test-PackageHash $destination $Package)) {
        Write-Host ("Download: {0} ({1:N1} MB)" -f $Package.file, ($Package.size / 1MB))
        $partial = "$destination.partial"
        try {
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            $request = [Net.HttpWebRequest]::Create($Package.url)
            $request.Timeout = 60000; $request.ReadWriteTimeout = 60000
            $response = $request.GetResponse()
            try {
                if ($response.ResponseUri.Scheme -ne 'https' -or $response.ContentLength -gt $Package.size) { throw 'Unexpected download response.' }
                $source = $response.GetResponseStream()
                $target = [IO.File]::Open($partial,[IO.FileMode]::Create,[IO.FileAccess]::Write,[IO.FileShare]::None)
                try {
                    $timer = [Diagnostics.Stopwatch]::StartNew()
                    $buffer = New-Object byte[] 65536; [long]$total = 0
                    while (($read = $source.Read($buffer,0,$buffer.Length)) -gt 0) {
                        $total += $read
                        if ($total -gt $Package.size -or $timer.Elapsed.TotalMinutes -gt 10) { throw 'Download exceeded its size/time limit.' }
                        $target.Write($buffer,0,$read)
                    }
                } finally { $target.Dispose(); $source.Dispose() }
            } finally { $response.Dispose() }
            if (-not (Test-PackageHash $partial $Package)) { throw "Download hash/size mismatch: $($Package.file)" }
            Move-Item -LiteralPath $partial -Destination $destination -Force
        } finally { if (Test-Path -LiteralPath $partial) { Remove-Item -LiteralPath $partial } }
    }
    if ($Package.PSObject.Properties.Name -contains 'signer') {
        $signature = Get-AuthenticodeSignature -LiteralPath $destination
        if ($signature.Status -ne 'Valid' -or -not $signature.SignerCertificate -or
            $signature.SignerCertificate.Subject -notlike "*$($Package.signer)*") { throw "Signature/publisher verification failed: $($Package.file)" }
    }
    return $destination
}

function Get-MainboardPlan([int]$NetRelease, [bool]$PawnInstalled) {
    return [pscustomobject]@{InstallDotNet=($NetRelease -lt 528040);InstallPawnIO=(-not $PawnInstalled);InstallLibrary=$true}
}

function Invoke-VerifiedInstaller([string]$Path, $Package, [string]$Arguments) {
    # Revalidate immediately before handing the signed upstream installer to UAC.
    if (-not (Test-PackageHash $Path $Package)) { throw 'Installer changed after download.' }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notlike "*$($Package.signer)*") { throw 'Installer signature changed.' }
    $start = @{FilePath=$Path;Verb='RunAs';Wait=$true;PassThru=$true}
    if ($Arguments) { $start.ArgumentList = $Arguments }
    $process = Start-Process @start
    if ($process.ExitCode -notin @(0,3010,1641)) { throw "Upstream installer exited with code $($process.ExitCode)." }
    return $process.ExitCode
}

function Install-LhmRuntime([string]$Archive, [string]$Destination, $Manifest, [string]$Cache) {
    if (-not (Test-PackageHash $Archive $Manifest.packages.lhm)) { throw 'Library archive changed.' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        foreach ($entry in $zip.Entries) {
            # Only flat runtime DLLs, never GUI executables, scripts or archive paths.
            if ($entry.FullName -notmatch '^[A-Za-z0-9_.-]+\.dll$' -or $entry.FullName -match '^(Aga\.Controls|OxyPlot.*|Microsoft\.Win32\.TaskScheduler)\.dll$') { continue }
            if ($entry.Length -gt 30MB) { throw 'Unexpected library entry size.' }
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $Destination $entry.FullName), $true)
        }
    } finally { $zip.Dispose() }
    $library = Join-Path $Destination 'LibreHardwareMonitorLib.dll'
    if (-not (Test-Path -LiteralPath $library) -or [Reflection.AssemblyName]::GetAssemblyName($library).Version.ToString() -ne '0.9.6.0') { throw 'LHM runtime version mismatch.' }
    foreach ($notice in $Manifest.notices) { Copy-Item -LiteralPath (Get-VerifiedPackage $notice $Cache) -Destination (Join-Path $Destination $notice.file) -Force }
    [IO.File]::WriteAllText((Join-Path $Destination 'SOURCES.txt'), "Unmodified upstream runtime: https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/v0.9.6`r`nEmbedded PawnIO.Modules source: https://github.com/namazso/PawnIO.Modules/releases/tag/0.1.6`r`nRuntime dependency identities: https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/v0.9.6/LibreHardwareMonitorLib/LibreHardwareMonitorLib.csproj`r`n")
}

Export-ModuleMember -Function Read-MainboardPackages,Test-PackageHash,Get-VerifiedPackage,Get-MainboardPlan,Invoke-VerifiedInstaller,Install-LhmRuntime
