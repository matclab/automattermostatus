use anyhow::Result;
//use tracing::debug;

/// Return the list of application name using the default microphone,
/// either via pulseaudio or alsa depending upon compilation option.
/// TODO for macOS
pub fn processes_owning_mic() -> Result<Vec<String>> {
    return Ok(vec![]);
}
