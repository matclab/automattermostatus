use crate::wifiscan::{WiFi, WifiError, WifiInterface};
use std::process::Command;

impl WiFi {
    /// Create linux `WiFi` interface
    pub fn new(interface: &str) -> Self {
        WiFi {
            interface: interface.to_owned(),
        }
    }
}

/// Wifi interface for linux operating system.
/// This provides basic functionalities for wifi interface.
impl WifiInterface for WiFi {
    /// Check if wireless network adapter is enabled.
    fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
        let output = Command::new("nmcli")
            .args(&["radio", "wifi"])
            .output()
            .map_err(WifiError::IoError)?;

        Ok(String::from_utf8_lossy(&output.stdout).contains("enabled"))
    }

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        let output = Command::new("nmcli")
            .args(&["-t", "-m", "tabular", "-f", "SSID", "device", "wifi"])
            .output()
            .map_err(WifiError::IoError)?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_owned();
        Ok(stdout.split('\n').map(str::to_string).collect())
    }
}
