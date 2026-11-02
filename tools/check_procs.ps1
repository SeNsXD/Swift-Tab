Get-Process | Where-Object { $_.ProcessName -like '*swift*' -or $_.ProcessName -like '*nowbar*' } | Select-Object Id, ProcessName, Path
Write-Host "--- Files in Programs\SwiftTab: ---"
Get-ChildItem 'C:\Users\Rishabh Kumar\AppData\Local\Programs\SwiftTab' -Force -ErrorAction SilentlyContinue
