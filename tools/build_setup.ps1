$ErrorActionPreference = "Stop"

$workspaceRoot = "D:\The Labs App\syro-nowbar-win11"
Set-Location $workspaceRoot

$env:NOWBAR_SETUP_PAYLOAD = "$workspaceRoot\target\quick"
$env:NOWBAR_VERSION = "1.0.0"

Write-Host "Building nowbar-setup with payload from $env:NOWBAR_SETUP_PAYLOAD ..."
cargo build --profile quick -p nowbar-setup

if (-not (Test-Path "$workspaceRoot\dist")) {
    New-Item -ItemType Directory -Path "$workspaceRoot\dist" -Force | Out-Null
}

Copy-Item "$workspaceRoot\target\quick\swifttab-setup.exe" "$workspaceRoot\dist\swifttab-setup.exe" -Force
Write-Host "Successfully built and copied dist\swifttab-setup.exe"
