# Builds SwiftTab.msix for Microsoft Store upload (unsigned: the Store signs it).
# Run from the project root:  powershell -ExecutionPolicy Bypass -File packaging\build-msix.ps1
$ErrorActionPreference = "Stop"
$root  = Split-Path -Parent $PSScriptRoot
$stage = Join-Path $root "target\msix-stage"
$out   = Join-Path $root "dist\SwiftTab.msix"

cargo build --release --bin swifttab --bin swifttab-settings
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item (Join-Path $root "target\release\swifttab.exe")          $stage
Copy-Item (Join-Path $root "target\release\swifttab-settings.exe") $stage
Copy-Item (Join-Path $PSScriptRoot "AppxManifest.xml") $stage
Copy-Item (Join-Path $PSScriptRoot "Assets") (Join-Path $stage "Assets") -Recurse

$kits = "C:\Program Files (x86)\Windows Kits\10\bin"
$tool = Get-ChildItem $kits -Recurse -Filter makeappx.exe |
        Where-Object { $_.FullName -match "\\x64\\" } | Sort-Object FullName | Select-Object -Last 1
if (-not $tool) { throw "makeappx.exe not found: install the Windows 10/11 SDK" }

New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
Remove-Item $out -ErrorAction SilentlyContinue
& $tool.FullName pack /d $stage /p $out /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed" }
Write-Host "Created $out"
