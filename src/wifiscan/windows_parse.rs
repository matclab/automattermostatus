pub(crate) fn extract_netsh_ssid(netsh_output: &str) -> Vec<String> {
    netsh_output
        .split('\n')
        .filter(|x| x.starts_with("SSID"))
        .map(|x| {
            x.split(':')
                .skip(1)
                .collect::<Vec<&str>>()
                .join(":")
                .trim()
                .to_owned()
        })
        .collect()
}

/// Check if a connected ethernet interface exists in `netsh interface show interface` output.
///
/// An interface is considered connected ethernet when its state is "Connected"
/// and its name is not the wifi interface, not "Loopback", and not "Bluetooth".
pub(crate) fn has_connected_ethernet(netsh_output: &str, wifi_interface: &str) -> bool {
    // Output format:
    // Admin State    State          Type             Interface Name
    // -------------------------------------------------------------------------
    // Enabled        Connected      Dedicated        Ethernet
    // Enabled        Connected      Dedicated        Wi-Fi
    netsh_output
        .lines()
        .skip_while(|line| !line.starts_with("---")) // Skip up to separator
        .skip(1) // Skip separator line itself
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let state = parts[1];
                let iface_name = parts[3..].join(" ");
                Some((state, iface_name))
            } else {
                None
            }
        })
        .any(|(state, iface_name)| {
            state == "Connected"
                && iface_name != wifi_interface
                && iface_name != "Loopback"
                && !iface_name.starts_with("Bluetooth")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        use anyhow::Result;
        #[test]
        fn detect_ethernet_when_connected() {
            let output = "\
Admin State    State          Type             Interface Name
-------------------------------------------------------------------------
Enabled        Connected      Dedicated        Ethernet
Enabled        Connected      Dedicated        Wi-Fi";
            assert!(has_connected_ethernet(output, "Wi-Fi"));
        }

        #[test]
        fn no_ethernet_when_only_wifi_connected() {
            let output = "\
Admin State    State          Type             Interface Name
-------------------------------------------------------------------------
Enabled        Disconnected   Dedicated        Ethernet
Enabled        Connected      Dedicated        Wi-Fi";
            assert!(!has_connected_ethernet(output, "Wi-Fi"));
        }

        #[test]
        fn no_ethernet_when_wifi_is_only_connection() {
            let output = "\
Admin State    State          Type             Interface Name
-------------------------------------------------------------------------
Enabled        Connected      Dedicated        Wi-Fi";
            assert!(!has_connected_ethernet(output, "Wi-Fi"));
        }

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

            assert_eq!(
                extract_netsh_ssid(res),
                ["SKYXXXXX", "SKYXXXXX", "XXXXX", "BTOpenzoneXXX"]
            );
            Ok(())
        }
    }
}
