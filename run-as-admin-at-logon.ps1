# Create a scheduled task that ensures `window-switcher.exe` runs with the admin privileges at logon

$programPath = $args[0]

if (-not $programPath) {
    $programPath = Read-Host "Enter the full path to window-switcher.exe"
} else {
    $programPath = (Resolve-Path $programPath).Path
}

# Check if the file exists at the provided path
if (-not (Test-Path $programPath)) {
    Write-Host "Invalid path at '$programPath'"
    exit 1
}

# Task name
$taskName = "WindowSwitcher"

# Current User
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$userId = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value

# Create a Scheduled Task Action
$action = New-ScheduledTaskAction -Execute $programPath

# Create a Scheduled Task Trigger 
$trigger = New-ScheduledTaskTrigger -AtLogon -User $user

# Set to run with highest privileges
$principal = New-ScheduledTaskPrincipal -UserId $userId -LogonType Interactive -RunLevel Highest

# Register the task
Register-ScheduledTask -TaskName $taskName -Action $action -TaskPath "\" `
    -Principal $principal -Trigger $trigger -Description "Run window-switcher.exe at logon" -Force