# Registers NitroMan in Windows Task Scheduler to run with Highest Privileges at logon.
# This eliminates UAC elevation prompts during Windows startup.

param(
    [string]$BinaryPath = "D:\Project\HermesAI\nitroman\dist\NitroMan.exe"
)

if (-not (Test-Path $BinaryPath)) {
    $BinaryPath = "D:\Project\HermesAI\nitroman\target\release\nitroman-gui.exe"
}

$TaskName = "NitroMan"

Write-Output "Configuring Task Scheduler for NitroMan..."
Write-Output "Target executable: $BinaryPath"

$Action = New-ScheduledTaskAction -Execute $BinaryPath
$Trigger = New-ScheduledTaskTrigger -AtLogOn
$Principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -RunLevel Highest
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit 0

# Remove existing task if already present
Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

# Register new elevated task
try {
    Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings | Out-Null
    Write-Output "Successfully registered elevated task '$TaskName'."
} catch {
    Write-Output "Failed to register task: $($_.Exception.Message)"
}
