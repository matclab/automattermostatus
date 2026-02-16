//! Implement wifi SSID scan for linux, windows and mac os.
// Mostly courtesy of https://github.com/tnkemdilim/wifi-rs

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod osx;
#[cfg(any(test, target_os = "macos"))]
mod osx_parse;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(any(test, target_os = "windows"))]
mod windows_parse;
// We include all modules for tests as tests do not depend upon specific platform
//#[cfg(test)]
//mod osx;

use crate::command::CommandRunner;
use std::{fmt, io};
use thiserror::Error;

/// Wireless network interface.
pub struct WiFi {
    /// wifi interface name (used on Windows for interface-specific commands)
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub interface: String,
    /// Command runner for executing system commands (enables mocking in tests)
    runner: Box<dyn CommandRunner>,
}

impl fmt::Debug for WiFi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WiFi")
            .field("interface", &self.interface)
            .finish()
    }
}

#[derive(Debug, Error)]
/// Error specific to `Wifi` struct.
pub enum WifiError {
    /// The specified wifi  is currently disabled. Try switching it on.
    #[error("Wifi is currently disabled")]
    WifiDisabled,
    /// The wifi interface interface failed to switch on.
    #[cfg(target_os = "windows")]
    #[error("Wifi interface failed to switch on")]
    InterfaceFailedToOn,
    #[allow(missing_docs)]
    #[error("Wifi IO Error")]
    IoError(#[from] io::Error),
}

/// Wifi interface for an operating system.
/// This provides basic functionalities for wifi interface.
pub trait WifiInterface: fmt::Debug {
    /// Check if the wifi interface on host machine is enabled.
    fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
        unimplemented!();
    }

    /// Return visible SSIDs
    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        unimplemented!();
    }
}
