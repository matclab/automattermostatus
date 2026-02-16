use super::osx_parse::extract_mic_in_use;
use crate::command::{CommandRunner, SystemCommandRunner};
use anyhow::Result;

/// Return the list of application name using the default microphone.
///
/// **macOS limitation**: `ioreg` can detect whether a USB microphone is in use
/// (via `IOAudioEngineState`), but it cannot identify *which* application is
/// using it. The returned list will contain `"unknown"` when any microphone is
/// active. A future improvement could use CoreAudio APIs to resolve actual
/// process names.
pub fn processes_owning_mic() -> Result<Vec<String>> {
    let runner = SystemCommandRunner;
    let mut res = Vec::new();
    let output = runner.run("ioreg", vec!["-l".into()])?;
    if extract_mic_in_use(&output) {
        res.push("unknown".to_string());
    }
    Ok(res)
}
