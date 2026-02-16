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

#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        use anyhow::Result;
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
