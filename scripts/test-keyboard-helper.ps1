param([switch]$CaptureLifecycle)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/build-keyboard-helper.ps1" -OutputDirectory "$PSScriptRoot/../target/keyboard-tests"
if (-not $CaptureLifecycle) { exit 0 }
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') { throw 'Hook lifecycle acceptance is restricted to a disposable GitHub Windows desktop.' }
$binary = [IO.Path]::GetFullPath("$PSScriptRoot/../target/keyboard-tests/freergb-keyboard-input.exe")
$start = [Diagnostics.ProcessStartInfo]::new($binary,'--listen')
$start.UseShellExecute=$false; $start.CreateNoWindow=$true
$start.RedirectStandardInput=$true; $start.RedirectStandardOutput=$true
$process = [Diagnostics.Process]::new(); $process.StartInfo=$start
$started=$false
try {
    $started=$process.Start()
    $ready=$process.StandardOutput.ReadLineAsync()
    if (-not $ready.Wait(3000)) { throw 'Keyboard helper did not report readiness.' }
    $value=$ready.Result | ConvertFrom-Json
    if ($value.version -ne 1 -or $value.kind -ne 'ready') { throw 'Invalid keyboard readiness record.' }
    # End immediately; never type, save or print captured events in this acceptance.
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(3000) -or $process.ExitCode -ne 0) { throw 'Keyboard hook did not cleanly release on EOF.' }
    Write-Output 'PASS: native hook startup and EOF cleanup on disposable runner. No input events recorded.'
} finally {
    if ($started -and -not $process.HasExited) { $process.Kill(); $null=$process.WaitForExit(3000) }
    $process.Dispose()
}
