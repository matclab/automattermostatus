#![warn(missing_docs)]
//! Automattermostatus main components and helper functions used by `main`
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::{collections::HashMap, time};
use tracing::{debug, error, info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

pub mod config;
pub mod mattermost;
pub mod micscan;
pub mod offtime;
pub mod state;
pub mod utils;
pub mod wifiscan;
pub use config::{Args, SecretType, WifiStatusConfig};
pub use mattermost::{BaseSession, LoggedSession, MMCutomStatus, Session};
use offtime::Off;
pub use state::{Cache, Location, State};
pub use wifiscan::{WiFi, WifiInterface};

/// Setup logging to stdout
/// (Tracing is a bit more involving to set up but will provide much more feature if needed)
pub fn setup_tracing(args: &Args) -> Result<()> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_new(args.verbose.get_level_filter()).unwrap();

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
        fs::create_dir_all(state_dir)
            .with_context(|| format!("Creating cache dir {:?}", &state_dir))?;
    } else {
        bail!("Internal Error, no `state_dir` configured");
    }

    state_file_name.push("automattermostatus.state");
    Ok(Cache::new(state_file_name))
}

/// Prepare a dictionnary of [`MMCustomStatus`] ready to be send to mattermost
/// server depending upon the location being found.
pub fn prepare_status(args: &Args) -> Result<HashMap<Location, MMCutomStatus>> {
    let mut res = HashMap::new();
    for s in &args.status {
        let sc: WifiStatusConfig = s.parse().with_context(|| format!("Parsing {}", s))?;
        debug!("Adding : {:?}", sc);
        res.insert(
            Location::Known(sc.wifi_string),
            MMCutomStatus::new(sc.text, sc.emoji),
        );
    }
    Ok(res)
}

/// Create [`Session`] according to `args.secret_type`.
pub fn create_session(args: &Args) -> Result<LoggedSession> {
    args.mm_url.as_ref().expect("Mattermost URL is not defined");
    args.secret_type
        .as_ref()
        .expect("Internal Error: secret_type is not defined");
    args.mm_secret.as_ref().expect("Secret is not defined");
    let mut session = Session::new(args.mm_url.as_ref().unwrap());
    let mut session: Box<dyn BaseSession> = match args.secret_type.as_ref().unwrap() {
        SecretType::Password => Box::new(session.with_credentials(
            args.mm_user.as_ref().unwrap(),
            args.mm_secret.as_ref().unwrap(),
        )),
        SecretType::Token => Box::new(session.with_token(args.mm_secret.as_ref().unwrap())),
    };
    let res = session.login();
    debug!("LoggedSession {:?}", res);
    res
}

/// Main application loop, looking for a known SSID and updating
/// mattermost custom status accordingly.
pub fn get_wifi_and_update_status_loop(
    args: Args,
    mut status_dict: HashMap<Location, MMCutomStatus>,
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
        error!("wifi is disabled");
    } else {
        info!("Wifi is enabled");
    }
    let mut session = create_session(&args)?;
    let mut micusage = &mut micscan::MicUsage::new();
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
                        mmstatus.expires_at(&args.expires_at);
                        if let Err(e) = state.update_status(
                            l.clone(),
                            Some(mmstatus),
                            &mut session,
                            &cache,
                            delay_duration.as_secs(),
                        ) {
                            error!("Fail to update status : {}", e)
                        }
                        break;
                    }
                }
            }
            if !found_ssid {
                debug!("Unknown wifi");
                if let Err(e) = state.update_status(
                    Location::Unknown,
                    None,
                    &mut session,
                    &cache,
                    delay_duration.as_secs(),
                ) {
                    error!("Fail to update status : {}", e)
                }
            }
        } else {
            // Send status for Off time (the one with empty wifi_substring).
            let off_location = Location::Known("".to_string());
            if let Some(offstatus) = status_dict.get_mut(&off_location) {
                debug!("Setting state for Offtime");
                if let Err(e) = state.update_status(
                    off_location,
                    Some(offstatus),
                    &mut session,
                    &cache,
                    delay_duration.as_secs(),
                ) {
                    error!("Fail to update status : {}", e)
                }
            }
        }
        micusage = micusage.update_dnd_status(&args, &mut session)?;
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
    use test_log::test; // Automatically trace tests

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
    use test_log::test; // Automatically trace tests

    #[test]
    fn prepare_expected_status() -> Result<()> {
        let args = Args {
            status: vec!["a::b::c", "d::e::f", "::off::off text"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            mm_secret: Some("AAA".to_string()),
            ..Default::default()
        };
        let res = prepare_status(&args)?;
        let mut expected: HashMap<state::Location, mattermost::MMCutomStatus> = HashMap::new();
        expected.insert(
            Location::Known("".to_string()),
            MMCutomStatus::new("off text".to_string(), "off".to_string()),
        );
        expected.insert(
            Location::Known("a".to_string()),
            MMCutomStatus::new("c".to_string(), "b".to_string()),
        );
        expected.insert(
            Location::Known("d".to_string()),
            MMCutomStatus::new("f".to_string(), "e".to_string()),
        );
        assert_eq!(res, expected);
        Ok(())
    }
}

#[cfg(test)]
mod create_session_should {
    use super::*;
    #[test]
    #[should_panic(expected = "Mattermost URL is not defined")]
    fn panic_when_mm_url_is_none() {
        let args = Args {
            status: vec!["a::b::c".to_string()],
            mm_secret: Some("AAA".to_string()),
            mm_url: None,
            ..Default::default()
        };
        let _res = create_session(&args);
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
