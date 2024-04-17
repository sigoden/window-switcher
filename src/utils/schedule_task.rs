use std::{os::windows::process::CommandExt, process::Command};

use anyhow::{bail, Result};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

#[derive(Debug)]
pub struct ScheduleTask {
    name: String,
    exe_path: String,
}

impl ScheduleTask {
    pub fn new(name: &str, exe_path: &str) -> Self {
        Self {
            name: name.to_string(),
            exe_path: exe_path.to_string(),
        }
    }

    pub fn create(&self) -> Result<()> {
        let output = Command::new("schtasks")
            .creation_flags(CREATE_NO_WINDOW.0) // CREATE_NO_WINDOW flag
            .args([
                "/create",
                "/tn",
                &self.name,
                "/tr",
                &self.exe_path,
                "/sc",
                "onlogon",
                "/rl",
                "highest",
                "/it",
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

    pub fn delete(&self) -> Result<()> {
        let output = Command::new("schtasks")
            .creation_flags(CREATE_NO_WINDOW.0) // CREATE_NO_WINDOW flag
            .args(["/delete", "/tn", &self.name, "/f"])
            .output()?;
        if !output.status.success() {
            bail!(
                "Fail to delete scheduled task, {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    pub fn exist(&self) -> Result<bool> {
        let output = Command::new("schtasks")
            .creation_flags(CREATE_NO_WINDOW.0) // CREATE_NO_WINDOW flag
            .args(["/query", "/tn", &self.name])
            .output()?;
        if output.status.success() {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
