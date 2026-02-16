use super::windows_parse::{extract_netsh_ssid, has_connected_ethernet};
use crate::command::SystemCommandRunner;
use crate::wifiscan::{WiFi, WifiError, WifiInterface};

impl WiFi {
    /// Create windows `WiFi` interface
    pub fn new(interface: &str) -> Self {
        WiFi {
            interface: interface.to_owned(),
            runner: Box::new(SystemCommandRunner),
        }
    }
}

/// Wifi interface for windows operating system.
/// This provides basic functionalities for wifi interface.
impl WifiInterface for WiFi {
    /// Check if wireless network adapter is enabled.
    fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
        let output = self
            .runner
            .run(
                "netsh",
                vec![
                    "wlan".into(),
                    "show".into(),
                    "interface".into(),
                    format!("name= \"{}\"", self.interface),
                ],
            )
            .map_err(|e| WifiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(!output.contains("There is no wireless interface"))
    }

    /// Check if an ethernet (wired) connection is currently active.
    fn is_ethernet_connected(&self) -> Result<bool, WifiError> {
        let output = self
            .runner
            .run(
                "netsh",
                vec!["interface".into(), "show".into(), "interface".into()],
            )
            .map_err(|e| WifiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(has_connected_ethernet(&output, &self.interface))
    }

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        let output = self
            .runner
            .run(
                "netsh",
                vec!["wlan".into(), "show".into(), "networks".into()],
            )
            .map_err(|e| WifiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        Ok(extract_netsh_ssid(&output))
    }
}
