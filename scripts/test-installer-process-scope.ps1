$ErrorActionPreference = 'Stop'
# Pure process objects: never enumerate or terminate a real process.
$root = Join-Path ([IO.Path]::GetTempPath()) ('freeRGB-scope-' + [guid]::NewGuid().ToString('N'))
function Fixture([string]$Path, [bool]$Wait = $true) {
    $p = [pscustomobject]@{Path=$Path;Handle=42;HasExited=$false;Killed=$false;Disposed=$false;Wait=$Wait}
    $p | Add-Member ScriptMethod Kill { $this.Killed=$true }
    $p | Add-Member ScriptMethod WaitForExit { param($timeout) return $this.Wait }
    $p | Add-Member ScriptMethod Dispose { $this.Disposed=$true }
    return $p
}
function Get-Process { param($Name,$ErrorAction) return $global:freeRGBMockProcessFixtures }
$owned = Fixture (Join-Path $root 'freergb-tray.exe')
$other = Fixture (Join-Path ($root + '-other') 'freergb-tray.exe')
$unknown = Fixture ''
$global:freeRGBMockProcessFixtures = @($owned,$other,$unknown)
& "$PSScriptRoot/../packaging/windows/stop-installed-tray.ps1" -InstallRoot $root
if (-not $owned.Killed -or $other.Killed -or $unknown.Killed) { throw 'Tray shutdown escaped its exact installation path.' }
if (-not $owned.Disposed -or -not $other.Disposed -or -not $unknown.Disposed) { throw 'Process handles were leaked.' }
$blocked = Fixture (Join-Path $root 'freergb-tray.exe') $false
$global:freeRGBMockProcessFixtures = @($blocked)
$rejected=$false
try { & "$PSScriptRoot/../packaging/windows/stop-installed-tray.ps1" -InstallRoot $root } catch { $rejected=$true }
if (-not $rejected -or -not $blocked.Disposed) { throw 'A failed shutdown must block maintenance and release its handle.' }
Remove-Variable freeRGBMockProcessFixtures -Scope Global
'PASS: exact-path tray shutdown, unrelated-copy preservation, handle cleanup and failed-stop rejection (mock processes only)'
