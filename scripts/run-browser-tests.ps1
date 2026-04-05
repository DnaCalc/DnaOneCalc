param(
    [switch]$Live
)

$ErrorActionPreference = "Stop"

& (Join-Path $PSScriptRoot "ensure-wasm-bindgen-runner.ps1") | Out-Null

if ($Live) {
    cargo test-browser-live -p dnaonecalc-host
} else {
    cargo test-browser -p dnaonecalc-host
}

exit $LASTEXITCODE
