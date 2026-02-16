use crate::command::SystemCommandRunner;
use crate::wifiscan::{WiFi, WifiError, WifiInterface};

impl WiFi {
    /// Create linux `WiFi` interface
    pub fn new(interface: &str) -> Self {
        WiFi {
            interface: interface.to_owned(),
            runner: Box::new(SystemCommandRunner),
        }
    }

    /// Create linux `WiFi` interface with a custom command runner (for testing)
    #[cfg(test)]
    pub fn with_runner(interface: &str, runner: Box<dyn crate::command::CommandRunner>) -> Self {
        WiFi {
            interface: interface.to_owned(),
            runner,
        }
    }
}

/// Wifi interface for linux operating system.
/// This provides basic functionalities for wifi interface.
impl WifiInterface for WiFi {
    /// Check if wireless network adapter is enabled.
    fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
        let output = self
            .runner
            .run("nmcli", vec!["radio".into(), "wifi".into()])
            .map_err(|e| WifiError::IoError(std::io::Error::other(e)))?;

        Ok(output.contains("enabled"))
    }

    /// Check if an ethernet (wired) connection is currently active.
    fn is_ethernet_connected(&self) -> Result<bool, WifiError> {
        let output = self
            .runner
            .run(
                "nmcli",
                vec![
                    "-t".into(),
                    "-f".into(),
                    "TYPE,STATE".into(),
                    "device".into(),
                ],
            )
            .map_err(|e| WifiError::IoError(std::io::Error::other(e)))?;

        Ok(output.lines().any(|line| line == "ethernet:connected"))
    }

    fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
        let output = self
            .runner
            .run(
                "nmcli",
                vec![
                    "-t".into(),
                    "-m".into(),
                    "tabular".into(),
                    "-f".into(),
                    "SSID".into(),
                    "device".into(),
                    "wifi".into(),
                ],
            )
            .map_err(|e| WifiError::IoError(std::io::Error::other(e)))?;
        Ok(output.split('\n').map(str::to_string).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::MockCommandRunner;

    #[test]
    fn visible_ssid_returns_parsed_output() {
        let mut mock = MockCommandRunner::new();
        mock.expect_run()
            .withf(|cmd, _args| cmd == "nmcli")
            .times(1)
            .returning(|_, _| Ok("HomeWifi\nOfficeNet\n".into()));

        let wifi = WiFi::with_runner("wlan0", Box::new(mock));
        let ssids = wifi.visible_ssid().unwrap();
        assert_eq!(ssids, vec!["HomeWifi", "OfficeNet", ""]);
    }

    #[test]
    fn is_wifi_enabled_returns_true_when_enabled() {
        let mut mock = MockCommandRunner::new();
        mock.expect_run()
            .withf(|cmd, _args| cmd == "nmcli")
            .times(1)
            .returning(|_, _| Ok("enabled\n".into()));

        let wifi = WiFi::with_runner("wlan0", Box::new(mock));
        assert!(wifi.is_wifi_enabled().unwrap());
    }

    #[test]
    fn is_ethernet_connected_returns_true_when_connected() {
        let mut mock = MockCommandRunner::new();
        mock.expect_run()
            .withf(|cmd, _args| cmd == "nmcli")
            .times(1)
            .returning(|_, _| Ok("wifi:connected\nethernet:connected\n".into()));

        let wifi = WiFi::with_runner("wlan0", Box::new(mock));
        assert!(wifi.is_ethernet_connected().unwrap());
    }

    #[test]
    fn is_ethernet_connected_returns_false_when_disconnected() {
        let mut mock = MockCommandRunner::new();
        mock.expect_run()
            .withf(|cmd, _args| cmd == "nmcli")
            .times(1)
            .returning(|_, _| Ok("wifi:connected\nethernet:disconnected\n".into()));

        let wifi = WiFi::with_runner("wlan0", Box::new(mock));
        assert!(!wifi.is_ethernet_connected().unwrap());
    }

    #[test]
    fn is_ethernet_connected_returns_false_when_no_ethernet_device() {
        let mut mock = MockCommandRunner::new();
        mock.expect_run()
            .withf(|cmd, _args| cmd == "nmcli")
            .times(1)
            .returning(|_, _| Ok("wifi:connected\nloopback:connected\n".into()));

        let wifi = WiFi::with_runner("wlan0", Box::new(mock));
        assert!(!wifi.is_ethernet_connected().unwrap());
    }

    #[test]
    fn is_wifi_enabled_returns_false_when_disabled() {
        let mut mock = MockCommandRunner::new();
        mock.expect_run()
            .withf(|cmd, _args| cmd == "nmcli")
            .times(1)
            .returning(|_, _| Ok("disabled\n".into()));

        let wifi = WiFi::with_runner("wlan0", Box::new(mock));
        assert!(!wifi.is_wifi_enabled().unwrap());
    }
}
