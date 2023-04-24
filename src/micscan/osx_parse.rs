use quick_xml::events::{BytesText, Event};
use quick_xml::Reader;
use tracing::{debug, error};

pub(crate) fn extract_mic_in_use(ioreg_output: &str) -> bool {
    usb_mic_in_use(ioreg_output)
}

fn node_has_engine_state(e: &BytesText, reader: &mut Reader<&[u8]>) -> bool {
    if e.unescape_and_decode(&reader).unwrap() == "IOAudioEngineState" {
        let mut buf = Vec::new();
        let _ = reader.read_event(&mut buf); // </key>
        let _ = reader.read_event(&mut buf); // <integer>
        if let Ok(Event::Text(e)) = reader.read_event(&mut buf) {
            if e.unescape_and_decode(&reader).unwrap() == "1" {
                debug!("Found IOAudioEngineState = 1");
                true
            } else {
                debug!("Found IOAudioEngineState != 1");
                false
            }
        } else {
            error!("Bad xml structure, expected text");
            false
        }
    } else {
        false
    }
}

pub(crate) fn usb_mic_in_use(ioreg_output: &str) -> bool {
    debug!("usb_mic_in_use");
    let mut reader = Reader::from_str(ioreg_output);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut dictlevel: i8 = 0;
    let mut sampleoffset_found: bool = false;
    let mut audioenginestate_found: bool = false;

    // Il faut alors rechercher un noeud qui contient à la fois une clé
    // IOAudioEngineInputSampleOffset (qui dit que c’est un flux entrant, donc un micro) avec une
    // valeur quelconque, et la clé IOAudioEngineState avec la valeur 1 qui dit qu’il est actif.
    // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name() {
                b"dict" => {
                    dictlevel += 1;
                    debug!(
                        "dict {} {} {}",
                        dictlevel, sampleoffset_found, audioenginestate_found
                    );
                }
                b"key" => {
                    debug!("key");
                    if let Ok(Event::Text(e)) = reader.read_event(&mut buf) {
                        if !audioenginestate_found {
                            audioenginestate_found = node_has_engine_state(&e, &mut reader);
                        }
                        if e.unescape_and_decode(&reader).unwrap()
                            == "IOAudioEngineInputSampleOffset"
                        {
                            debug!("Found IOAudioEngineInputSampleOffset");
                            sampleoffset_found = true;
                        }
                    }
                }
                _ => (),
            },
            Ok(Event::End(ref e)) => match e.name() {
                b"dict" => {
                    dictlevel -= 1;
                    debug!(
                        "End dict {} {} {}",
                        dictlevel, sampleoffset_found, audioenginestate_found
                    );
                    if dictlevel == 1 {
                        if sampleoffset_found && audioenginestate_found {
                            return true;
                        }
                        // Reset boolean trigger for next flux
                        sampleoffset_found = false;
                        audioenginestate_found = false;
                    }
                }
                _ => (),
            },
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
    }
    // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
    buf.clear();
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        use anyhow::Result;
        use test_log::test;
        #[test]
        fn find_mic_connected() -> Result<()> {
            let res = include_str!("macscanmic.xml");
            assert_eq!(usb_mic_in_use(res), true);
            Ok(())
        }
    }
}
