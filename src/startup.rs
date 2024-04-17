use anyhow::Result;
use windows::core::{w, PCWSTR};

use crate::utils::{get_exe_path, RegKey, ScheduleTask};

const TASK_NAME: &str = "WindowSwitcher";
const HKEY_RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const HKEY_NAME: PCWSTR = w!("Window Switcher");

#[derive(Default)]
pub struct Startup {
    pub is_enable: bool,
    pub task: Option<ScheduleTask>,
    pub exe_path: Vec<u16>,
}

impl Startup {
    pub fn init(is_admin: bool) -> Result<Self> {
        let exe_path = get_exe_path();
        let (task, is_enable) = if is_admin {
            let exe_path_str = String::from_utf16_lossy(&exe_path);
            let task = ScheduleTask::new(TASK_NAME, &exe_path_str);
            let is_enable = task.exist()?;
            (Some(task), is_enable)
        } else {
            (None, reg_is_enable(&exe_path)?)
        };
        Ok(Self {
            is_enable,
            exe_path,
            task,
        })
    }

    pub fn toggle(&mut self) -> Result<()> {
        if self.is_enable {
            match &self.task {
                Some(task) => task.delete()?,
                None => reg_disable()?,
            }
            self.is_enable = false;
        } else {
            match &self.task {
                Some(task) => task.create()?,
                None => reg_enable(&self.exe_path)?,
            };
            self.is_enable = true;
        }
        Ok(())
    }
}

fn reg_key() -> Result<RegKey> {
    RegKey::new_hkcu(HKEY_RUN, HKEY_NAME)
}

fn reg_is_enable(exe_path: &[u16]) -> Result<bool> {
    let key = reg_key()?;
    let value = match key.get_value()? {
        Some(value) => value,
        None => return Ok(false),
    };
    Ok(value == exe_path)
}

fn reg_enable(exe_path: &[u16]) -> Result<()> {
    let key = reg_key()?;
    let path = unsafe { exe_path.align_to::<u8>().1 };
    key.set_value(path)?;
    Ok(())
}

fn reg_disable() -> Result<()> {
    let key = reg_key()?;
    key.delete_value()?;
    Ok(())
}
