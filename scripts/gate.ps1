param(
    [switch]$Fast,
    [string[]]$Package
)
$ErrorActionPreference = 'Stop'
if ($Fast -and -not $Package) {
    throw 'Fast mode requires -Package with the affected Cargo package names (include affected consumers).'
}
if ($Package -and -not $Fast) { throw '-Package requires -Fast; omit both for the full workspace gate.' }
function Invoke-Cargo([string[]]$Arguments) {
    Write-Host "cargo $($Arguments -join ' ')"
    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) { throw "cargo failed with exit code $LASTEXITCODE" }
}
Push-Location "$PSScriptRoot/.."
try {
    $scope = @('--workspace')
    if ($Fast) {
        $scope = @()
        foreach ($name in $Package) { $scope += @('-p', $name) }
    }
    Invoke-Cargo @('fmt', '--all', '--check')
    Invoke-Cargo (@('clippy') + $scope + @('--all-targets', '--locked', '--', '-D', 'warnings'))
    Invoke-Cargo (@('nextest', 'run') + $scope + @('--locked'))
    Invoke-Cargo (@('test', '--doc') + $scope + @('--locked'))
    # Keep CI's synthetic process fixtures: these must be explicitly feature-enabled.
    if (-not $Fast -or $Package -contains 'freergb-daemon') {
        Invoke-Cargo @('test', '-p', 'freergb-daemon', '--features', 'cooling-fixture', '--test', 'cooling_guard_process', '--locked')
        Invoke-Cargo @('test', '-p', 'freergb-daemon', '--features', 'bus-fixture', '--test', 'bus_process', '--locked')
    }
    if (-not $Fast -or $Package -contains 'freergb-modules' -or $Package -contains 'freergb-cooling-virtual') {
        Invoke-Cargo @('build', '-p', 'freergb-cooling-virtual', '--locked')
        Invoke-Cargo @('test', '-p', 'freergb-modules', '--test', 'cooling_e2e', '--locked', '--', '--ignored')
    }
} finally {
    Pop-Location
}
