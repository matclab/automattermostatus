use super::windows_parse::extract_netsh_ssid;
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

    /// Turn on the wireless network adapter.
    fn turn_on(&self) -> Result<(), WifiError> {
        Command::new("netsh")
            .args(&[
                "interface",
                "set",
                "interface",
                &format!("name= \"{}\"", self.interface),
                "ENABLED",
            ])
            .output()
            .map_err(|err| WifiError::IoError(err))?;

        Ok(())
    }

    /// Turn off the wireless network adapter.
    fn turn_off(&self) -> Result<(), WifiError> {
        let _output = Command::new("netsh")
            .args(&[
                "interface",
                "set",
                "interface",
                &format!("name= \"{}\"", self.interface),
                "DISABLED",
            ])
            .output()
            .map_err(|err| WifiError::IoError(err))?;

        Ok(())
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
