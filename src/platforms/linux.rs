use crate::platforms::{WiFi, WifiError, WifiInterface};
use std::process::Command;

impl WiFi {
    pub fn new(interface: &str) -> Self {
        WiFi {
            connection: None,
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
            .map_err(|err| WifiError::IoError(err))?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .replace(" ", "")
            .replace("\n", "")
            .contains("enabled"))
    }

    /// Turn on the wireless network adapter.
    fn turn_on(&self) -> Result<(), WifiError> {
        Command::new("nmcli")
            .args(&["radio", "wifi", "on"])
            .output()
            .map_err(|err| WifiError::IoError(err))?;

        Ok(())
    }

    /// Turn off the wireless network adapter.
    fn turn_off(&self) -> Result<(), WifiError> {
        Command::new("nmcli")
            .args(&["radio", "wifi", "off"])
            .output()
            .map_err(|err| WifiError::IoError(err))?;

        Ok(())
    }

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        let output = Command::new("nmcli")
            .args(&["-t", "-m", "tabular", "-f", "SSID", "device", "wifi"])
            .output()
            .map_err(|err| WifiError::IoError(err))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_owned();
        Ok(stdout.split("\n").map(str::to_string).collect())
    }
}
