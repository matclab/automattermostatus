use anyhow::Result;
use procfs::process::Process;
use tracing::debug;

use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(feature = "pulseaudio")]
use pulsectl::controllers::{AppControl, SourceController};

#[cfg(feature = "pulseaudio")]
pub fn pulseaudio_processes_using_mic() -> Result<Vec<String>> {
    let mut res = Vec::new();
    // create handler that calls functions on playback devices and apps
    let mut handler = SourceController::create()?;
    for app in handler.list_applications()? {
        if let Some(name) = app.proplist.get_str("application.process.binary") {
            res.push(name);
        }
    }
    Ok(res)
}

// Select directories whose names start with the given parameter
pub fn select_directories<P>(current_dir: P, prefix: &str) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let mut directories: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        let metadata = fs::metadata(&path)?;
        let starts_with_prefix = path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.starts_with(prefix))
            .unwrap_or(false);
        debug!(
            "{:?} {:?} {:?}",
            path,
            metadata.is_dir(),
            starts_with_prefix
        );
        if metadata.is_dir() && starts_with_prefix {
            directories.push(path);
        }
    }
    debug!("{:?}", directories);
    Ok(directories)
}

fn pid_from_status_file(lines: &str) -> Result<i32> {
    let pid: String = lines
        .split('\n')
        .filter(|x| x.starts_with("owner_pid"))
        .map(|x| {
            x.split(':')
                .skip(1)
                .collect::<Vec<&str>>()
                .join("")
                .trim()
                .to_owned()
        })
        .collect();
    pid.parse().map_err(anyhow::Error::msg)
}

pub fn alsa_processes_owning_mic() -> Result<Vec<String>> {
    let current_dir = "/proc/asound";

    let mut res = Vec::new();

    for card in select_directories(current_dir, "card")? {
        debug!("Exploring card {:?}", card);
        for pcm in select_directories(card, "pcm")? {
            debug!("Exploring pcm {:?}", pcm);
            for sub in select_directories(pcm, "sub")? {
                let mut status = sub;
                status.push("status");
                debug!("Checking status of {:?}", status);
                let lines = fs::read_to_string(status.clone())?;
                if lines.contains("owner_pid") {
                    let pid = pid_from_status_file(&lines)?;
                    let procinfo = Process::new(pid)?;
                    let process_name = procinfo.cmdline()?[0].to_owned();
                    res.push(process_name);
                }
            }
        }
    }
    debug!("Process owning mic : {:?}", res);
    Ok(res)
}

/// Return the list of application name using the default microphone,
/// either via pulseaudio or alsa depending upon compilation option.
pub fn processes_owning_mic() -> Result<Vec<String>> {
    #[cfg(feature = "pulseaudio")]
    if let Ok(res) = pulseaudio_processes_using_mic() {
        return Ok(res);
    }
    alsa_processes_owning_mic()
}

#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        //use anyhow::Result;
        #[test]
        fn extract_expected_pid() -> Result<()> {
            let res = r#"
state: RUNNING
owner_pid   : 3700
trigger_time: 1101.147470817
tstamp      : 1104.422995946
delay       : 144
avail       : 8048
avail_max   : 8048
-----
hw_ptr      : 157296
appl_ptr    : 157440
"#;

            assert_eq!(pid_from_status_file(res)?, 3700);
            Ok(())
        }
    }
}
