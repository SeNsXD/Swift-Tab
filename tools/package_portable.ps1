$workspaceRoot = "D:\The Labs App\syro-nowbar-win11"
$tempStage = "$workspaceRoot\dist\portable_staging"

if (Test-Path $tempStage) {
    Remove-Item -Path $tempStage -Recurse -Force
}
New-Item -ItemType Directory -Path $tempStage -Force | Out-Null

Copy-Item "$workspaceRoot\target\quick\swifttab.exe" "$tempStage\swifttab.exe"
Copy-Item "$workspaceRoot\target\quick\swifttab-settings.exe" "$tempStage\swifttab-settings.exe"
Copy-Item "$workspaceRoot\README.md" "$tempStage\README.md"
Copy-Item "$workspaceRoot\LICENSE" "$tempStage\LICENSE"

$zipPath = "$workspaceRoot\dist\SwiftTab-v1.0.0-portable.zip"
if (Test-Path $zipPath) {
    Remove-Item $zipPath -Force
}

Compress-Archive -Path "$tempStage\*" -DestinationPath $zipPath -Force
Remove-Item -Path $tempStage -Recurse -Force

Write-Host "Created $zipPath"
