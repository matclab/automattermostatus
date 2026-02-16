use quick_xml::events::Event;
use quick_xml::Reader;
use tracing::error;

pub(crate) fn extract_airport_ssid(airport_output: &str) -> Vec<String> {
    let mut reader = Reader::from_str(airport_output);
    reader.config_mut().trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"key" {
                    if let Ok(Event::Text(e)) = reader.read_event_into(&mut buf) {
                        if let Ok(key_content) = e.xml_content() {
                            if key_content == "SSID_STR" {
                                let _ = reader.read_event(); // </key>
                                let _ = reader.read_event(); // <string>
                                if let Ok(Event::Text(e)) = reader.read_event_into(&mut buf) {
                                    if let Ok(ssid) = e.xml_content() {
                                        txt.push(ssid.to_string());
                                    } else {
                                        error!("Failed to read SSID_STR xml content");
                                    }
                                } else {
                                    error!("Bad xml structure")
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                error!(
                    "XML parse error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                );
                break;
            }
            _ => (), // There are several other `Event`s we do not consider here
        }
    }
    // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
    buf.clear();
    txt
}

/// Check if any active ethernet interface exists in `ifconfig` output.
///
/// An interface is considered active ethernet when it is NOT `lo0`, NOT the
/// wifi interface, has `UP` in its flags, and has an `inet` address.
pub(crate) fn has_active_ethernet(ifconfig_output: &str, wifi_interface: &str) -> bool {
    let mut current_iface = "";
    let mut is_up = false;
    let mut has_inet = false;

    for line in ifconfig_output.lines() {
        // Interface header line: "en0: flags=8863<UP,BROADCAST,...>"
        if !line.starts_with('\t') && !line.starts_with(' ') && line.contains(": flags=") {
            // Check previous interface before moving to next
            if !current_iface.is_empty()
                && current_iface != "lo0"
                && current_iface != wifi_interface
                && is_up
                && has_inet
            {
                return true;
            }
            // Parse new interface name
            current_iface = line.split(':').next().unwrap_or("");
            is_up = line.contains("UP");
            has_inet = false;
        } else if line.trim_start().starts_with("inet ") {
            has_inet = true;
        }
    }

    // Check the last interface
    !current_iface.is_empty()
        && current_iface != "lo0"
        && current_iface != wifi_interface
        && is_up
        && has_inet
}

#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        use anyhow::Result;
        #[test]
        fn detect_ethernet_when_present() {
            let output = "\
lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384
\tinet 127.0.0.1 netmask 0xff000000
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet 192.168.1.10 netmask 0xffffff00 broadcast 192.168.1.255
en1: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet 10.0.0.5 netmask 0xffffff00 broadcast 10.0.0.255";
            // en0 is wifi, en1 is ethernet
            assert!(has_active_ethernet(output, "en0"));
        }

        #[test]
        fn no_ethernet_when_only_wifi_and_loopback() {
            let output = "\
lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384
\tinet 127.0.0.1 netmask 0xff000000
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet 192.168.1.10 netmask 0xffffff00 broadcast 192.168.1.255";
            assert!(!has_active_ethernet(output, "en0"));
        }

        #[test]
        fn no_ethernet_when_interface_is_down() {
            let output = "\
lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384
\tinet 127.0.0.1 netmask 0xff000000
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet 192.168.1.10 netmask 0xffffff00 broadcast 192.168.1.255
en1: flags=8822<BROADCAST,SMART,SIMPLEX,MULTICAST> mtu 1500
\tinet 10.0.0.5 netmask 0xffffff00 broadcast 10.0.0.255";
            // en1 does not have UP flag
            assert!(!has_active_ethernet(output, "en0"));
        }

        #[test]
        fn extract_expected_ssid() -> Result<()> {
            let res = include_str!("macscan.xml");
            assert_eq!(
                extract_airport_ssid(res),
                ["NEUF_5EE4", "FreeWifi_secure", "SFR_6A68", "NEUF_5EE4"]
            );
            Ok(())
        }
    }
}
