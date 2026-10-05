#Requires -Version 7
# One command for the local floor: format, clippy, tests, and a release build.
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo clippy --locked --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo test --locked --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Same remap the package build uses. Prefixes stay out of the script text.
$flags = New-Object System.Collections.Generic.List[string]
function Add-Remap([string]$From, [string]$To) {
    if (-not $From) { return }
    $flags.Add("--remap-path-prefix=${From}=${To}")
    $forward = $From -replace '\\', '/'
    if ($forward -ne $From) { $flags.Add("--remap-path-prefix=${forward}=${To}") }
}
$repo = (Get-Location).Path
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
Add-Remap $cargoHome 'cargo-home'
Add-Remap $rustupHome 'rustup'
Add-Remap $repo 'runforge'
Add-Remap $env:USERPROFILE 'home'
$env:RUSTFLAGS = $flags -join ' '
cargo build --locked --release -p runforge
$code = $LASTEXITCODE
Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
if ($code -ne 0) { exit $code }

Write-Output 'verify: fmt, clippy, test, and the release build passed'
