$workspaceRoot = "D:\The Labs App\syro-nowbar-win11"
$tempStage = "$workspaceRoot\dist\portable_staging"

$version = (Select-String -Path "$workspaceRoot\Cargo.toml" -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

if (Test-Path $tempStage) {
    Remove-Item -Path $tempStage -Recurse -Force
}
New-Item -ItemType Directory -Path $tempStage -Force | Out-Null

$srcDir = if (Test-Path "$workspaceRoot\dist\swifttab.exe") { "$workspaceRoot\dist" } elseif (Test-Path "$workspaceRoot\target\dist\swifttab.exe") { "$workspaceRoot\target\dist" } else { "$workspaceRoot\target\quick" }

Copy-Item "$srcDir\swifttab.exe" "$tempStage\swifttab.exe" -Force
Copy-Item "$srcDir\swifttab-settings.exe" "$tempStage\swifttab-settings.exe" -Force
Copy-Item "$workspaceRoot\README.md" "$tempStage\README.md" -Force
Copy-Item "$workspaceRoot\LICENSE" "$tempStage\LICENSE" -Force

$zipPath = "$workspaceRoot\dist\SwiftTab-v$version-portable.zip"
if (Test-Path $zipPath) {
    Remove-Item $zipPath -Force
}

Compress-Archive -Path "$tempStage\*" -DestinationPath $zipPath -Force
Remove-Item -Path $tempStage -Recurse -Force

Write-Host "Created $zipPath"
