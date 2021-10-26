use quick_xml::events::Event;
use quick_xml::Reader;
use tracing::{error};

pub(crate) fn extract_airport_ssid(airport_output: &str) -> Vec<String> {
    let mut reader = Reader::from_str(airport_output);
    reader.trim_text(true);

    let mut txt = Vec::new();
    let mut buf = Vec::new();

    // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"key" => {
                        if let Ok(Event::Text(e)) = reader.read_event(&mut buf) {
                            if e.unescape_and_decode(&reader).unwrap() == "SSID_STR" {
                                let _ =reader.read_event(&mut buf); // </key>
                                let _ =reader.read_event(&mut buf); // </string>
                                if let Ok(Event::Text(e)) = reader.read_event(&mut buf) {
                                    txt.push(e.unescape_and_decode(&reader).unwrap());
                                } else {
                                    error!("Bad xml structure")
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
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
