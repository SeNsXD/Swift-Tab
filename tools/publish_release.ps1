param(
    [string]$Repo = "SeNsXD/Swift-Tab",
    [string]$Tag = "v1.0.2",
    [string]$Title = "Swift Tab 1.0.2",
    [string]$BodyPath = "dist\RELEASE_BODY.md"
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Net.Http

Write-Host "Getting GitHub credentials from Git Credential Manager..."
$credsText = "protocol=https`nhost=github.com`n" | git credential fill
$token = ""
foreach ($line in ($credsText -split "`n")) {
    if ($line -match "^password=(.*)$") {
        $token = $matches[1].Trim()
    }
}

if (-not $token) {
    throw "Failed to retrieve GitHub token from git credential manager."
}

$fullBodyPath = (Resolve-Path $BodyPath).Path
$body = [System.IO.File]::ReadAllText($fullBodyPath, [System.Text.Encoding]::UTF8)

$headers = @{
    "Authorization" = "Bearer $token"
    "Accept" = "application/vnd.github+json"
    "User-Agent" = "SwiftTab-Release-Publisher"
}

# 1. Check if release already exists
$release = $null
try {
    $existing = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/tags/$Tag" -Headers $headers -Method Get
    Write-Host "Found existing release for $Tag (ID: $($existing.id)). Updating..."
    $updatePayload = @{
        name = $Title
        body = $body
        draft = $false
        prerelease = $false
    } | ConvertTo-Json
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/$($existing.id)" -Headers $headers -Method Patch -Body ([System.Text.Encoding]::UTF8.GetBytes($updatePayload)) -ContentType "application/json; charset=utf-8"
} catch {
    Write-Host "Release for $Tag does not exist yet. Creating..."
    $createPayload = @{
        tag_name = $Tag
        name = $Title
        body = $body
        draft = $false
        prerelease = $false
    } | ConvertTo-Json
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases" -Headers $headers -Method Post -Body ([System.Text.Encoding]::UTF8.GetBytes($createPayload)) -ContentType "application/json; charset=utf-8"
}

Write-Host "Release created/updated successfully: $($release.html_url)"
$releaseId = $release.id

# 2. Upload assets
$distDir = Resolve-Path "dist"
$assetsToUpload = @(
    "swifttab-setup.exe",
    "SwiftTab-v1.0.2-portable.zip",
    "SwiftTab.msix",
    "swifttab.exe",
    "swifttab-settings.exe",
    "SHA256SUMS.txt"
)

# Fetch existing assets for this release
$currentAssets = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/$releaseId/assets" -Headers $headers -Method Get

$httpClient = [System.Net.Http.HttpClient]::new()
$httpClient.DefaultRequestHeaders.Add("Authorization", "Bearer $token")
$httpClient.DefaultRequestHeaders.Add("User-Agent", "SwiftTab-Release-Publisher")
$httpClient.DefaultRequestHeaders.Add("Accept", "application/vnd.github+json")
$httpClient.Timeout = [TimeSpan]::FromMinutes(10)

foreach ($assetName in $assetsToUpload) {
    $filePath = Join-Path $distDir $assetName
    if (-not (Test-Path $filePath)) {
        Write-Warning "File not found: $filePath, skipping..."
        continue
    }

    # Delete if asset already exists in release
    $existingAsset = $currentAssets | Where-Object { $_.name -eq $assetName }
    if ($existingAsset) {
        Write-Host "Deleting existing asset $($assetName) (ID: $($existingAsset.id))..."
        Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/assets/$($existingAsset.id)" -Headers $headers -Method Delete | Out-Null
    }

    $fileBytes = [System.IO.File]::ReadAllBytes($filePath)
    $content = [System.Net.Http.ByteArrayContent]::new($fileBytes)
    $content.Headers.ContentType = [System.Net.Http.Headers.MediaTypeHeaderValue]::Parse("application/octet-stream")

    $uploadUrl = "https://uploads.github.com/repos/$Repo/releases/$releaseId/assets?name=$assetName"
    Write-Host "Uploading $assetName ($([math]::Round($fileBytes.Length / 1MB, 2)) MB)..."

    $response = $httpClient.PostAsync($uploadUrl, $content).GetAwaiter().GetResult()
    if (-not $response.IsSuccessStatusCode) {
        $respBody = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        throw "Failed to upload $assetName : $($response.StatusCode) - $respBody"
    }

    Write-Host "Successfully uploaded $assetName"
}

$httpClient.Dispose()

# 3. Final verification
Write-Host "Verifying published release assets..."
$finalAssets = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/$releaseId/assets" -Headers $headers -Method Get
Write-Host "Release assets:"
$finalAssets | Select-Object name, size, download_count | Format-Table -AutoSize

Write-Host "Release published at: $($release.html_url)"
