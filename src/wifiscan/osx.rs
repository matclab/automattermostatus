use super::osx_parse::extract_airport_ssid;
use crate::wifiscan::{WiFi, WifiError, WifiInterface};
use std::process::Command;

impl WiFi {
    pub fn new(interface: &str) -> Self {
        WiFi {
            connection: None,
            interface: interface.to_owned(),
        }
    }
}

/// Wifi interface for osx operating system.
/// This provides basic functionalities for wifi interface.
impl WifiInterface for WiFi {
    fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
        let output = Command::new("networksetup")
            .args(&["radio", "wifi"])
            .output()
            .map_err(|err| WifiError::IoError(err))?;

        Ok(String::from_utf8_lossy(&output.stdout).contains("enabled"))
    }

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        let output = Command::new(
            "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/A/Resources/airport ",
        )
        .args(&["scan"])
        .output()
        .map_err(|err| WifiError::IoError(err))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_owned();
        Ok(extract_airport_ssid(&stdout))
    }
}
