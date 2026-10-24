param([Parameter(Mandatory=$true)][string]$InstallRoot)
$ErrorActionPreference = 'Stop'
$expected = [IO.Path]::GetFullPath((Join-Path $InstallRoot 'freergb-tray.exe'))
foreach ($process in @(Get-Process -Name freergb-tray -ErrorAction SilentlyContinue)) {
    try {
        # Capture the handle before inspecting the path, so Kill targets this
        # process object rather than a recycled numeric process ID.
        $null = $process.Handle
        if ($process.HasExited) { continue }
        if ($process.Path -and [string]::Equals([IO.Path]::GetFullPath($process.Path), $expected, [StringComparison]::OrdinalIgnoreCase)) {
            $process.Kill()
            if (-not $process.WaitForExit(5000)) { throw 'Installed tray did not stop.' }
        }
    } finally { $process.Dispose() }
}
