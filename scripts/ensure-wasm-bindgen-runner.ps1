param(
    [string]$Version = "0.2.117",
    [switch]$PrintPath
)

$ErrorActionPreference = "Stop"

function Get-WasmBindgenTargetTriple {
    $os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

    if ($os -match "Windows") {
        return "x86_64-pc-windows-msvc"
    }

    if ($os -match "Darwin|macOS|Mac OS") {
        if ($arch -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
            return "aarch64-apple-darwin"
        }

        return "x86_64-apple-darwin"
    }

    return "x86_64-unknown-linux-musl"
}

function Get-WasmBindgenRunnerName {
    $os = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    if ($os -match "Windows") {
        return "wasm-bindgen-test-runner.exe"
    }

    return "wasm-bindgen-test-runner"
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$targetTriple = Get-WasmBindgenTargetTriple
$runnerName = Get-WasmBindgenRunnerName
$toolRoot = Join-Path $repoRoot ".tools\wasm-bindgen\$Version\$targetTriple"
$archiveName = "wasm-bindgen-$Version-$targetTriple.tar.gz"
$archivePath = Join-Path $toolRoot $archiveName
$downloadUrl = "https://github.com/rustwasm/wasm-bindgen/releases/download/$Version/$archiveName"

if (-not (Test-Path $toolRoot)) {
    New-Item -ItemType Directory -Path $toolRoot -Force | Out-Null
}

$runnerPath = Get-ChildItem -Path $toolRoot -Recurse -File -Filter $runnerName -ErrorAction SilentlyContinue |
    Select-Object -First 1 -ExpandProperty FullName

if (-not $runnerPath) {
    if (-not (Test-Path $archivePath)) {
        Write-Host "Downloading $downloadUrl"
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
    }

    tar -xzf $archivePath -C $toolRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to extract wasm-bindgen archive from $archivePath"
    }

    $runnerPath = Get-ChildItem -Path $toolRoot -Recurse -File -Filter $runnerName -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
}

if (-not $runnerPath) {
    throw "Could not locate $runnerName under $toolRoot after extraction"
}

if ($PrintPath) {
    $runnerPath
}
