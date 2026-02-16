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

pub mod command;
pub mod config;
pub mod mattermost;
pub mod micscan;
pub mod offtime;
pub mod state;
pub mod utils;
pub mod wifiscan;
pub use config::{AppConfig, Args, SecretType, WifiStatusConfig};
pub use mattermost::{BaseSession, LoggedSession, MMCustomStatus, Session};
use offtime::Off;
pub use state::{Cache, Location, State};
pub use wifiscan::{WiFi, WifiInterface};

/// Setup logging to stdout
/// (Tracing is a bit more involving to set up but will provide much more feature if needed)
pub fn setup_tracing(args: &Args) -> Result<()> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer =
        EnvFilter::try_new(args.verbose.get_level_filter()).context("Initializing log filter")?;

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
pub fn prepare_status(config: &AppConfig) -> Result<HashMap<Location, MMCustomStatus>> {
    let mut res = HashMap::new();
    for s in &config.wifi.statuses {
        let sc: WifiStatusConfig = s.parse().with_context(|| format!("Parsing {s}"))?;
        debug!("Adding : {:?}", sc);
        res.insert(
            Location::Known(sc.wifi_string),
            MMCustomStatus::new(sc.text, sc.emoji),
        );
    }
    Ok(res)
}

/// Create [`Session`] according to the mattermost configuration.
pub fn create_session(config: &AppConfig) -> Result<LoggedSession> {
    let mm = &config.mattermost;
    let delay_duration = time::Duration::new(config.schedule.delay.into(), 0);
    let mut session = Session::new(&mm.url);
    let mut session: Box<dyn BaseSession> = match mm.secret_type {
        SecretType::Password => {
            let mm_user = mm.user.as_ref().context("Mattermost user is not defined")?;
            Box::new(session.with_credentials(mm_user, &mm.secret))
        }
        SecretType::Token => Box::new(session.with_token(&mm.secret)),
    };
    loop {
        let res = session.login();
        if let Ok(session) = res {
            debug!("LoggedSession {:?}", session);
            return Ok(session);
        } else {
            error!("Failed to access mattermost API {:?}", res);
            sleep(delay_duration);
        }
    }
}

/// Process a single iteration of the main loop.
///
/// This function encapsulates the logic for one polling cycle: checking wifi SSIDs,
/// updating mattermost status, and handling off-time. It is extracted from the main
/// loop to enable unit testing.
///
/// **Note on mutable state**: [`State`] and [`micscan::MicUsage`] are currently
/// managed as independent mutable references. If a third signal source is added
/// (e.g. calendar, GPS), consider unifying them behind a single `AppState` struct
/// to keep the coordination logic manageable.
#[allow(clippy::too_many_arguments)]
pub fn process_one_iteration(
    wifi: &dyn WifiInterface,
    micusage: &mut micscan::MicUsage,
    state: &mut State,
    session: &mut LoggedSession,
    config: &AppConfig,
    status_dict: &mut HashMap<Location, MMCustomStatus>,
    cache: &Cache,
    delay_secs: u64,
) -> Result<()> {
    if !config.schedule.is_off_time() {
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
                    mmstatus.expires_at(&config.schedule.expires_at);
                    if let Err(e) =
                        state.update_status(l.clone(), Some(mmstatus), session, cache, delay_secs)
                    {
                        error!("Fail to update status : {}", e)
                    }
                    break;
                }
            }
        }
        if !found_ssid {
            debug!("Unknown wifi");
            if let Err(e) = state.update_status(Location::Unknown, None, session, cache, delay_secs)
            {
                error!("Fail to update status : {}", e)
            }
        }
    } else {
        // Send status for Off time (the one with empty wifi_substring).
        let off_location = Location::Known("".to_string());
        if let Some(offstatus) = status_dict.get_mut(&off_location) {
            debug!("Setting state for Offtime");
            if let Err(e) =
                state.update_status(off_location, Some(offstatus), session, cache, delay_secs)
            {
                error!("Fail to update status : {}", e)
            }
        }
    }
    micusage.update_dnd_status(&config.mic.app_names, session);
    Ok(())
}

/// Main application loop, looking for a known SSID and updating
/// mattermost custom status accordingly.
pub fn get_wifi_and_update_status_loop(
    config: AppConfig,
    mut status_dict: HashMap<Location, MMCustomStatus>,
) -> Result<()> {
    let cache = get_cache(Some(config.state_dir.clone())).context("Reading cached state")?;
    let mut state = State::new(&cache).context("Creating cache")?;
    let delay_duration = time::Duration::new(config.schedule.delay.into(), 0);
    let wifi = WiFi::new(&config.wifi.interface_name);
    if !wifi
        .is_wifi_enabled()
        .context("Checking if wifi is enabled")?
    {
        error!("wifi is disabled");
    } else {
        info!("Wifi is enabled");
    }
    let mut session = create_session(&config)?;
    let mut micusage = micscan::MicUsage::new();
    loop {
        process_one_iteration(
            &wifi,
            &mut micusage,
            &mut state,
            &mut session,
            &config,
            &mut status_dict,
            &cache,
            delay_duration.as_secs(),
        )?;
        if config.schedule.delay == 0 {
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
            status: ["a::b::c", "d::e::f", "::off::off text"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            mm_secret: Some("AAA".to_string()),
            ..Default::default()
        };
        let config = args.validate()?;
        let res = prepare_status(&config)?;
        let mut expected: HashMap<state::Location, mattermost::MMCustomStatus> = HashMap::new();
        expected.insert(
            Location::Known("".to_string()),
            MMCustomStatus::new("off text".to_string(), "off".to_string()),
        );
        expected.insert(
            Location::Known("a".to_string()),
            MMCustomStatus::new("c".to_string(), "b".to_string()),
        );
        expected.insert(
            Location::Known("d".to_string()),
            MMCustomStatus::new("f".to_string(), "e".to_string()),
        );
        assert_eq!(res, expected);
        Ok(())
    }
}

#[cfg(test)]
mod validate_should {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn error_when_mm_url_is_none() -> Result<()> {
        let args = Args {
            status: vec!["a::b::c".to_string()],
            mm_secret: Some("AAA".to_string()),
            mm_url: None,
            ..Default::default()
        };
        match args.validate() {
            Ok(_) => Err(anyhow!("Expected an error")),
            Err(e) => {
                assert!(e.to_string().contains("mm_url"), "Unexpected error: {}", e);
                Ok(())
            }
        }
    }

    #[test]
    fn error_when_delay_is_none() -> Result<()> {
        let args = Args {
            status: vec!["a::b::c".to_string()],
            mm_secret: Some("AAA".to_string()),
            delay: None,
            ..Default::default()
        };
        match args.validate() {
            Ok(_) => Err(anyhow!("Expected an error")),
            Err(e) => {
                assert!(e.to_string().contains("Delay"), "Unexpected error: {}", e);
                Ok(())
            }
        }
    }

    #[test]
    fn succeed_with_valid_args() -> Result<()> {
        let args = Args {
            mm_secret: Some("secret".to_string()),
            ..Default::default()
        };
        let config = args.validate()?;
        assert_eq!(config.mattermost.url, "https://mattermost.example.com");
        assert_eq!(config.schedule.delay, 60);
        assert!(!config.wifi.interface_name.is_empty());
        Ok(())
    }
}

#[cfg(test)]
mod process_one_iteration_should {
    use super::*;
    use crate::config::{MattermostConfig, MicConfig, ScheduleConfig, WifiConfig};
    use crate::offtime::OffDays;
    use crate::wifiscan::WifiError;
    use httpmock::prelude::*;
    use mktemp::Temp;
    use test_log::test;

    /// Mock wifi that returns a configurable list of SSIDs
    #[derive(Debug)]
    struct MockWifi {
        ssids: Vec<String>,
    }

    impl WifiInterface for MockWifi {
        fn is_wifi_enabled(&self) -> Result<bool, WifiError> {
            Ok(true)
        }
        fn visible_ssid(&self) -> Result<Vec<String>, WifiError> {
            Ok(self.ssids.clone())
        }
    }

    fn test_config(server_url: &str, state_dir: PathBuf) -> AppConfig {
        AppConfig {
            mattermost: MattermostConfig {
                url: server_url.to_string(),
                user: None,
                secret_type: SecretType::Token,
                secret: "token".to_string(),
            },
            schedule: ScheduleConfig {
                begin: None,
                end: None,
                expires_at: None,
                delay: 0,
                offdays: OffDays::default(),
            },
            wifi: WifiConfig {
                interface_name: "wlan0".to_string(),
                statuses: vec!["HomeNet::house::At home".to_string()],
            },
            mic: MicConfig { app_names: vec![] },
            state_dir,
        }
    }

    #[test]
    fn update_status_when_known_wifi_is_visible() -> Result<()> {
        let server = MockServer::start();
        let temp = Temp::new_dir().unwrap().to_path_buf();
        let config = test_config(&server.url(""), temp.clone());

        // Mock the login endpoint
        let login_mock = server.mock(|expect, resp_with| {
            expect
                .method(GET)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me");
            resp_with
                .status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({"id":"user_id"}));
        });

        // Mock the custom status endpoint
        let status_mock = server.mock(|expect, resp_with| {
            expect
                .method(PUT)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me/status/custom");
            resp_with.status(200).body("ok");
        });

        let wifi = MockWifi {
            ssids: vec!["HomeNet".to_string(), "NeighborWifi".to_string()],
        };

        let cache = Cache::new(temp.join("automattermostatus.state"));
        let mut state = State::new(&cache)?;
        let mut session = Session::new(&server.url("")).with_token("token").login()?;
        let mut micusage = micscan::MicUsage::new();
        let mut status_dict = prepare_status(&config)?;

        process_one_iteration(
            &wifi,
            &mut micusage,
            &mut state,
            &mut session,
            &config,
            &mut status_dict,
            &cache,
            0,
        )?;

        login_mock.assert();
        status_mock.assert();
        Ok(())
    }

    #[test]
    fn not_update_status_when_wifi_is_unknown() -> Result<()> {
        let server = MockServer::start();
        let temp = Temp::new_dir().unwrap().to_path_buf();
        let config = test_config(&server.url(""), temp.clone());

        // Login mock
        let login_mock = server.mock(|expect, resp_with| {
            expect
                .method(GET)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me");
            resp_with
                .status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({"id":"user_id"}));
        });

        // Status endpoint should NOT be called
        let status_mock = server.mock(|expect, resp_with| {
            expect.method(PUT).path("/api/v4/users/me/status/custom");
            resp_with.status(200).body("ok");
        });

        let wifi = MockWifi {
            ssids: vec!["UnknownWifi".to_string()],
        };

        let cache = Cache::new(temp.join("automattermostatus.state"));
        let mut state = State::new(&cache)?;
        let mut session = Session::new(&server.url("")).with_token("token").login()?;
        let mut micusage = micscan::MicUsage::new();
        let mut status_dict = prepare_status(&config)?;

        process_one_iteration(
            &wifi,
            &mut micusage,
            &mut state,
            &mut session,
            &config,
            &mut status_dict,
            &cache,
            0,
        )?;

        login_mock.assert();
        // The status endpoint should not have been called
        status_mock.assert_hits(0);
        Ok(())
    }
}
