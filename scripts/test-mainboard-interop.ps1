param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
if (-not $SkipBuild) { & "$PSScriptRoot/test-mainboard-control.ps1" }
$binary = [IO.Path]::GetFullPath("$PSScriptRoot/../target/native-control-tests/native-control-tests.exe")
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Test-only native fixture is missing.' }
$previous = $env:FREERGB_NATIVE_SESSION_FIXTURE
try {
    $env:FREERGB_NATIVE_SESSION_FIXTURE = $binary
    cargo test -p freergb-daemon --test native_session_interop --locked -- --ignored
    if ($LASTEXITCODE -ne 0) { throw 'Rust/C# native session interoperability failed.' }
} finally { $env:FREERGB_NATIVE_SESSION_FIXTURE = $previous }
