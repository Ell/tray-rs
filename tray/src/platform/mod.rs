#[cfg(target_os = "windows")]
#[path = "win32/mod.rs"]
mod platform;

use crate::TrayMenu;
use core::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub use platform::Platform;

type Result<T> = std::result::Result<T, TrayPlatformError>;

#[derive(Debug, Clone)]
pub struct TrayPlatformError {
    pub details: String,
}

impl TrayPlatformError {
    fn new(msg: &str) -> Self {
        Self {
            details: msg.to_string(),
        }
    }
}

impl std::fmt::Display for TrayPlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Tray Error: {}", self.details)
    }
}

impl std::error::Error for TrayPlatformError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub trait TrayPlatform {
    fn update(&self) -> Result<()>;
    fn quit(&self) -> Result<()>;
    fn run(&self) -> Result<()>;
    fn init(&mut self, menu: Arc<RefCell<TrayMenu>>) -> Result<()>;
}
