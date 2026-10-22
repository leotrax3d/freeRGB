param([string]$OutputDirectory = "$PSScriptRoot/../target/release")
$ErrorActionPreference = 'Stop'
$destination = [System.IO.Path]::GetFullPath($OutputDirectory)
Push-Location "$PSScriptRoot/../apps/desktop"
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw 'Desktop dependency installation failed' }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw 'Desktop frontend build failed' }
    cargo build --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Desktop Rust build failed' }
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item -LiteralPath 'src-tauri/target/release/freergb-desktop.exe' -Destination $destination
} finally { Pop-Location }
