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

use std::{fmt, io};
use thiserror::Error;

#[derive(Debug)]
pub struct Connection {
    pub ssid: String,
}

/// Wireless network interface for linux operating system.
#[derive(Debug)]
pub struct WiFi {
    #[allow(dead_code)]
    pub connection: Option<Connection>,
    #[allow(dead_code)]
    pub interface: String,
}

#[derive(Debug, Error)]
pub enum WifiError {
    /// The specified wifi  is currently disabled. Try switching it on.
    #[error("Wifi is currently disabled")]
    WifiDisabled,
    /// The wifi interface interface failed to switch on.
    #[cfg(target_os = "windows")]
    #[error("Wifi interface failed to switch on")]
    InterfaceFailedToOn,
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

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        unimplemented!();
    }
}
