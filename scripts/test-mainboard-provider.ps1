$ErrorActionPreference = 'Stop'
$directory = [System.IO.Path]::GetFullPath("$PSScriptRoot/../target/mainboard-tests")
./scripts/build-mainboard-helper.ps1 -OutputDirectory $directory
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$source = (Resolve-Path -LiteralPath "$PSScriptRoot/../providers/mainboard/tests/Fixture.cs").Path
$fixture = Join-Path $directory 'fixture.exe'
& $compiler /nologo /target:exe /platform:x64 /warnaserror+ "/out:$fixture" $source
if ($LASTEXITCODE -ne 0) { throw 'Provider transport fixture compilation failed.' }
$previousHelper = $env:FREERGB_TEST_MAINBOARD_HELPER
$previousFixture = $env:FREERGB_TEST_MAINBOARD_FIXTURE
try {
    $env:FREERGB_TEST_MAINBOARD_HELPER = Join-Path $directory 'freergb-mainboard.exe'
    $env:FREERGB_TEST_MAINBOARD_FIXTURE = $fixture
    cargo test -p freergb-daemon mainboard::tests::helper_transport -- --ignored
    if ($LASTEXITCODE -ne 0) { throw 'Mainboard provider process acceptance failed.' }
} finally {
    $env:FREERGB_TEST_MAINBOARD_HELPER = $previousHelper
    $env:FREERGB_TEST_MAINBOARD_FIXTURE = $previousFixture
}
