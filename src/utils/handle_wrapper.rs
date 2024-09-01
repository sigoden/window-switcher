use windows::Win32::Foundation::{CloseHandle, HANDLE};

#[derive(Debug, Clone, Default)]
pub struct HandleWrapper {
    handle: HANDLE,
}

impl HandleWrapper {
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }
    pub fn get_handle(&self) -> HANDLE {
        self.handle
    }
    pub fn get_handle_mut(&mut self) -> &mut HANDLE {
        &mut self.handle
    }
}

impl Drop for HandleWrapper {
    fn drop(&mut self) {
        if self.handle.is_invalid() {
            return;
        }
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}
