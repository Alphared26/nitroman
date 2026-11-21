# Removes NitroMan from Windows Task Scheduler.

$TaskName = "NitroMan"

Write-Output "Removing '$TaskName' from Task Scheduler..."
try {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction Stop
    Write-Output "Successfully unregistered '$TaskName'."
} catch {
    Write-Output "Task was not registered or could not be removed: $($_.Exception.Message)"
}
