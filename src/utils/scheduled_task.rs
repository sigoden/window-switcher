use std::{os::windows::process::CommandExt, process::Command};

use anyhow::{bail, Result};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

pub fn create_scheduled_task(name: &str, exe_path: &str) -> Result<()> {
    let output = Command::new("schtasks")
        .creation_flags(CREATE_NO_WINDOW.0) // CREATE_NO_WINDOW flag
        .args([
            "/create", "/tn", name, "/tr", exe_path, "/sc", "onlogon", "/rl", "highest", "/it",
            "/f",
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Fail to create scheduled task, {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn delete_scheduled_task(name: &str) -> Result<()> {
    let output = Command::new("schtasks")
        .creation_flags(CREATE_NO_WINDOW.0) // CREATE_NO_WINDOW flag
        .args(["/delete", "/tn", name, "/f"])
        .output()?;
    if !output.status.success() {
        bail!(
            "Fail to delete scheduled task, {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn exist_scheduled_task(name: &str) -> Result<bool> {
    let output = Command::new("schtasks")
        .creation_flags(CREATE_NO_WINDOW.0) // CREATE_NO_WINDOW flag
        .args(["/query", "/tn", name])
        .output()?;
    if output.status.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}
