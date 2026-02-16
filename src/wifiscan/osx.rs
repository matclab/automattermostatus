use super::osx_parse::extract_airport_ssid;
use crate::command::SystemCommandRunner;
use crate::wifiscan::{WiFi, WifiError, WifiInterface};

impl WiFi {
    /// Create MacOS `WiFi` interface
    pub fn new(interface: &str) -> Self {
        WiFi {
            interface: interface.to_owned(),
            runner: Box::new(SystemCommandRunner),
        }
    }
}

/// Wifi interface for osx operating system.
/// This provides basic functionalities for wifi interface.
impl WifiInterface for WiFi {
    fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
        let output = self
            .runner
            .run("networksetup", vec!["radio".into(), "wifi".into()])
            .map_err(|e| WifiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(output.contains("enabled"))
    }

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        let output = self
            .runner
            .run(
                "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/A/Resources/airport ",
                vec!["scan".into()],
            )
            .map_err(|e| WifiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        Ok(extract_airport_ssid(&output))
    }
}
