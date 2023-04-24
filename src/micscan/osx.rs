use super::osx_parse::extract_mic_in_use;
use anyhow::Result;
use std::process::Command;
//use tracing::debug;

/// Return the list of application name using the default microphone,
/// either via pulseaudio or alsa depending upon compilation option.
/// TODO for macOS
pub fn processes_owning_mic() -> Result<Vec<String>> {
    let mut res = Vec::new();
    let output = Command::new("ioreg").args(&["-l"]).output()?;
    if extract_mic_in_use(&String::from_utf8_lossy(&output.stdout)) {
        res.push("unknown".to_string());
    }
    Ok(res)
}
