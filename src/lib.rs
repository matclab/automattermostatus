#![warn(missing_docs)]
//! Automattermostatus main components and helper functions:
//! - [`config`]: allow to configure the application from file and command line,
//! - [`mattermost`]:  updating custom status on a mattermost instance,
//! - [`state`]: persistent application state (essentially the location),
//! - [`wifiscan`]: wifi scanning for linux, macos and windows
//! - [`offtime`]: management of time when no custom status shall be send
//! - [`utils`]: some simple helper functions to parse time string
//!
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::thread::sleep;
use std::{collections::HashMap, time};
use tracing::{debug, info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

pub mod config;
pub mod mattermost;
pub mod offtime;
pub mod state;
pub mod utils;
pub mod wifiscan;
pub use config::{Args, WifiStatusConfig};
pub use mattermost::MMStatus;
use offtime::Off;
pub use state::{Cache, Location, State};
pub use wifiscan::{WiFi, WifiInterface};

/// Setup logging to stdout
/// (Tracing is a bit more involving to set up but will provide much more feature if needed)
pub fn setup_tracing(args: &Args) -> Result<()> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_new(args.verbose.get_level_filter().to_string()).unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
    Ok(())
}

/// Return a [`Cache`] used to persist state.
pub fn get_cache(dir: Option<PathBuf>) -> Result<Cache> {
    let mut state_file_name: PathBuf;
    if let Some(ref state_dir) = dir {
        state_file_name = PathBuf::from(state_dir);
    } else {
        bail!("Internal Error, no `state_dir` configured");
    }

    state_file_name.push("automattermostatus.state");
    Ok(Cache::new(state_file_name))
}

/// Prepare a dictionnary of [`Status`] ready to be send to mattermost
/// server depending upon the location being found.
pub fn prepare_status(args: &Args) -> Result<HashMap<Location, MMStatus>> {
    let mut res = HashMap::new();
    for s in &args.status {
        let sc: WifiStatusConfig = s.parse().with_context(|| format!("Parsing {}", s))?;
        debug!("Adding : {:?}", sc);
        res.insert(
            Location::Known(sc.wifi_string),
            MMStatus::new(
                sc.text,
                sc.emoji,
                args.mm_url
                    .as_ref()
                    .expect("Mattermost URL is not defined")
                    .clone(),
                args.mm_token
                    .clone()
                    .expect("Mattermost private access token is not defined"),
            ),
        );
    }
    Ok(res)
}

/// Main application loop, looking for a known SSID and updating
/// mattermost custom status accordingly.
pub fn get_wifi_and_update_status_loop(
    args: Args,
    mut status_dict: HashMap<Location, MMStatus>,
) -> Result<()> {
    let cache = get_cache(args.state_dir.to_owned()).context("Reading cached state")?;
    let mut state = State::new(&cache).context("Creating cache")?;
    let delay_duration = time::Duration::new(
        args.delay
            .expect("Internal error: args.delay shouldn't be None")
            .into(),
        0,
    );
    let wifi = WiFi::new(
        &args
            .interface_name
            .clone()
            .expect("Internal error: args.interface_name shouldn't be None"),
    );
    if !wifi
        .is_wifi_enabled()
        .context("Checking if wifi is enabled")?
    {
        bail!("wifi is disabled");
    } else {
        info!("Wifi is enabled");
    }
    loop {
        if !&args.is_off_time() {
            let ssids = wifi.visible_ssid().context("Getting visible SSIDs")?;
            debug!("Visible SSIDs {:#?}", ssids);
            let mut found_ssid = false;
            // Search for known wifi in visible ssids
            for (l, mmstatus) in status_dict.iter_mut() {
                if let Location::Known(wifi_substring) = l {
                    if ssids.iter().any(|x| x.contains(wifi_substring)) {
                        if wifi_substring.is_empty() {
                            debug!("We do not match against empty SSID reserved for off time");
                            continue;
                        }
                        debug!("known wifi '{}' detected", wifi_substring);
                        found_ssid = true;
                        let mmstatus = mmstatus.clone().expires_at(&args.expires_at);
                        state
                            .update_status(l.clone(), Some(&mmstatus), &cache)
                            .context("Updating status")?;
                        break;
                    }
                }
            }
            if !found_ssid {
                debug!("Unknown wifi");
                state
                    .update_status(Location::Unknown, None, &cache)
                    .context("Updating status")?;
            }
        } else {
            // Send status for Off time (the one with empty wifi_substring).
            let off_location = Location::Known("".to_string());
            if let Some(offstatus) = status_dict.get_mut(&off_location) {
                debug!("Setting state for Offtime");
                state
                    .update_status(off_location, Some(offstatus), &cache)
                    .context("Updating status")?;
            }
        }
        if let Some(0) = args.delay {
            break;
        } else {
            sleep(delay_duration);
        }
    }
    Ok(())
}

#[cfg(test)]
mod get_cache_should {
    use super::*;
    use anyhow::anyhow;

    #[test]
    //#[should_panic(expected = "Internal error, no `state_dir` configured")]
    fn panic_when_called_with_none() -> Result<()> {
        match get_cache(None) {
            Ok(_) => Err(anyhow!("Expected an error")),
            Err(e) => {
                assert_eq!(e.to_string(), "Internal Error, no `state_dir` configured");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod prepare_status_should {
    use super::*;

    #[test]
    fn prepare_expected_status() -> Result<()> {
        let args = Args {
            status: vec!["a::b::c", "d::e::f", "::off::off text"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            mm_token: Some("AAA".to_string()),
            ..Default::default()
        };
        let res = prepare_status(&args)?;
        let token = "AAA";
        let uri = "https://mattermost.example.com";
        let mut expected: HashMap<state::Location, mattermost::MMStatus> = HashMap::new();
        expected.insert(
            Location::Known("".to_string()),
            MMStatus::new(
                "off text".to_string(),
                "off".to_string(),
                uri.to_string(),
                token.to_string(),
            ),
        );
        expected.insert(
            Location::Known("a".to_string()),
            MMStatus::new(
                "c".to_string(),
                "b".to_string(),
                uri.to_string(),
                token.to_string(),
            ),
        );
        expected.insert(
            Location::Known("d".to_string()),
            MMStatus::new(
                "f".to_string(),
                "e".to_string(),
                uri.to_string(),
                token.to_string(),
            ),
        );
        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    #[should_panic(expected = "Mattermost URL is not defined")]
    fn panic_when_mm_url_is_none() {
        let args = Args {
            status: vec!["a::b::c".to_string()],
            mm_token: Some("AAA".to_string()),
            mm_url: None,
            ..Default::default()
        };
        let _res = prepare_status(&args);
    }

    #[test]
    #[should_panic(expected = "Mattermost private access token is not defined")]
    fn panic_when_mm_token_is_none() {
        let args = Args {
            status: vec!["a::b::c".to_string()],
            mm_token: None,
            ..Default::default()
        };
        let _res = prepare_status(&args);
    }
}

#[cfg(test)]
mod main_loop_should {
    use super::*;

    #[test]
    #[should_panic(expected = "Internal error: args.delay shouldn't be None")]
    fn panic_when_args_delay_is_none() {
        let args = Args {
            status: vec!["a::b::c".to_string()],
            delay: None,
            ..Default::default()
        };
        let _res = get_wifi_and_update_status_loop(args, HashMap::new());
    }
}
