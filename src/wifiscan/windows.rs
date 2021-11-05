use super::windows_parse::extract_netsh_ssid;
use crate::wifiscan::{WiFi, WifiError, WifiInterface};
use std::process::Command;

impl WiFi {
    pub fn new(interface: &str) -> Self {
        WiFi {
            interface: interface.to_owned(),
        }
    }
}

/// Wifi interface for windows operating system.
/// This provides basic functionalities for wifi interface.
impl WifiInterface for WiFi {
    /// Check if wireless network adapter is enabled.
    fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
        let output = Command::new("netsh")
            .args(&[
                "wlan",
                "show",
                "interface",
                &format!("name= \"{}\"", self.interface),
            ])
            .output()
            .map_err(|err| WifiError::IoError(err))?;

        Ok(!String::from_utf8_lossy(&output.stdout).contains("There is no wireless interface"))
    }

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        let output = Command::new("netsh")
            .args(&["wlan", "show", "networks"])
            .output()
            .map_err(|err| WifiError::IoError(err))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_owned();
        Ok(extract_netsh_ssid(&stdout))
    }
}
