use crate::platforms::{ WifiError, WifiInterface, WiFi};
use std::process::Command;

const WINDOWS_INTERFACE: &'static str = "Wireless Network Connection";


impl WiFi {
    pub fn new(interface: &str) -> Self {
        WiFi {
            connection: None,
            interface: interface.to_owned(),
        }
    }
}
fn extract_netsh_ssid(netsh_output: &str) -> Vec<String> {
    netsh_output.split("\n")
        .filter(|x| x.starts_with("SSID"))
        .map(|x| x.split(":").skip(1).collect::<Vec<&str>>().join(":").trim().to_owned())
        .collect()
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
                &format!("name= \"{}\"", WINDOWS_INTERFACE),
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
                &format!("name= \"{}\"", WINDOWS_INTERFACE),
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
                &format!("name= \"{}\"", WINDOWS_INTERFACE),
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

#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        use anyhow::Result;
        #[test]
        fn extract_expected_ssid() -> Result<()> {
            let res = r#"
Interface name : Wireless Network Connection
There are 22 networks currently visible.

SSID 1 : SKYXXXXX
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP

SSID 2 : SKYXXXXX
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP

SSID 3 : XXXXX
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP

SSID 4 : BTOpenzoneXXX
    Network type            : Infrastructure
    Authentication          : Open
    Encryption              : None
"#;

            assert_eq!(extract_netsh_ssid(res), ["SKYXXXXX", "SKYXXXXX", "XXXXX", "BTOpenzoneXXX"]);
            Ok(())
        }
    }
}
