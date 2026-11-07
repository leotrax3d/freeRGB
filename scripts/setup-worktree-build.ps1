param(
    [string]$TargetDir,
    [string]$CacheDir = 'E:/freergb-build/sccache',
    [switch]$UseDefaultLinker
)
$ErrorActionPreference = 'Stop'
Push-Location "$PSScriptRoot/.."
try {
    $root = (Get-Location).Path
    $config = Join-Path $root '.cargo/config.toml'
    git check-ignore -q -- .cargo/config.toml
    if ($LASTEXITCODE -ne 0) {
        throw 'First add .cargo/config.toml to your local Git exclude file (git rev-parse --git-path info/exclude).'
    }
    $content = if (Test-Path -LiteralPath $config) { [IO.File]::ReadAllText($config) } else { '' }
    if (-not $TargetDir -and $content -notmatch '(?m)^\s*target-dir\s*=') {
        $TargetDir = 'E:/freergb-build/' + (Split-Path $root -Leaf)
    }
    function Set-TomlKey([string]$Section, [string]$Key, [string]$Value) {
        $header = '(?m)^\[' + [regex]::Escape($Section) + '\][ \t]*\r?$'
        $match = [regex]::Match($script:content, $header)
        $line = "$Key = $Value`n"
        if (-not $match.Success) {
            $script:content += "`n[$Section]`n$line"
            return
        }
        $start = $match.Index + $match.Length
        $next = [regex]::Match($script:content.Substring($start), '(?m)^\[')
        $end = if ($next.Success) { $start + $next.Index } else { $script:content.Length }
        $body = $script:content.Substring($start, $end - $start)
        $keyPattern = '(?m)^[ \t]*' + [regex]::Escape($Key) + '[ \t]*=[^\r\n]*(?:\r?\n|$)'
        $body = [regex]::Replace($body, $keyPattern, '')
        $script:content = $script:content.Substring(0, $start) + "`n" + $line + $body.TrimStart("`r", "`n") + $script:content.Substring($end)
    }
    if ($TargetDir) { Set-TomlKey 'build' 'target-dir' ("'" + $TargetDir.Replace('\', '/') + "'") }
    Set-TomlKey 'build' 'rustc-wrapper' '"sccache"'
    $linker = if ($UseDefaultLinker) { 'link.exe' } else { 'rust-lld' }
    Set-TomlKey 'target.x86_64-pc-windows-msvc' 'linker' ('"' + $linker + '"')
    Set-TomlKey 'env' 'SCCACHE_DIR' ('{ value = "' + $CacheDir.Replace('\', '/') + '", force = true }')
    $content = [regex]::Replace($content, '(?:\r?\n){2,}(?=\[)', "`n")
    New-Item -ItemType Directory -Force (Split-Path $config) | Out-Null
    [IO.File]::WriteAllText($config, $content, [Text.UTF8Encoding]::new($false))
    Write-Output "Configured $config"
} finally {
    Pop-Location
}
