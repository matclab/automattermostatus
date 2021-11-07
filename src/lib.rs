#![warn(missing_docs)]
//! Automattermostatus main components and helper functions:
//! - `config`: allow to configure the application from file and command line,
//! - `mattermost`:  updating custom status on a mattermost instance,
//! - `state`: persistent application state (essentially the location),
//! - `wifiscan`: wifi scanning for linux, macos and windows
//! - `offtime`: management of time when no custom status shall be send
//!
use anyhow::{bail, Context, Result};
use directories_next::ProjectDirs;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::{collections::HashMap, fs, time};
use tracing::{debug, info};
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

/// Return a `Cache` used to persist state.
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

/// Update `args.mm_token` with the standard output of
/// `args.mm_token_cmd` if defined.
pub fn update_token_with_command(mut args: Args) -> Result<Args> {
    if let Some(command) = &args.mm_token_cmd {
        let params =
            shell_words::split(command).context("Splitting mm_token_cmd into shell words")?;
        debug!("Running command {}", command);
        let output = Command::new(&params[0])
            .args(&params[1..])
            .output()
            .context(format!("Error when running {}", &command))?;
        let token = String::from_utf8_lossy(&output.stdout);
        if token.len() == 0 {
            bail!("command '{}' returns nothing", &command);
        }
        // /!\ Do not spit secret on stdout on released binary.
        //debug!("setting token to {}", token);
        args.mm_token = Some(token.to_string());
    }
    Ok(args)
}

/// Prepare a dictionnary of `Status` ready to be send to mattermost
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
            )
            .expires_at(&args.expires_at),
        );
    }
    Ok(res)
}

/// Merge with precedence default [`Args`], config file and command line parameters.
pub fn merge_config_and_params(args: &Args) -> Result<Args> {
    let default_args = Args::default();
    debug!("default Args : {:#?}", default_args);
    let conf_dir = ProjectDirs::from("net", "clabaut", "automattermostatus")
        .expect("Unable to find a project dir")
        .config_dir()
        .to_owned();
    fs::create_dir_all(&conf_dir).with_context(|| format!("Creating conf dir {:?}", &conf_dir))?;
    let conf_file = conf_dir.join("automattermostatus.toml");
    if !conf_file.exists() {
        info!("Write {:?} default config file", &conf_file);
        fs::write(&conf_file, toml::to_string(&Args::default())?)
            .unwrap_or_else(|_| panic!("Unable to write default config file {:?}", conf_file));
    }

    let config_args: Args = Figment::from(Toml::file(&conf_file)).extract()?;
    debug!("config Args : {:#?}", config_args);
    debug!("parameter Args : {:#?}", args);
    // Merge config Default → Config File → command line args
    let res = Figment::from(Serialized::defaults(Args::default()))
        .merge(Toml::file(&conf_file))
        .merge(Serialized::defaults(args))
        .extract()
        .context("Merging configuration file and parameters")?;
    debug!("Merged config and parameters : {:#?}", res);
    Ok(res)
}

/// Main application loop, looking for a known SSID and updating
/// mattermost custom status accordingly.
pub fn get_wifi_and_update_status_loop(
    args: Args,
    status_dict: HashMap<Location, MMStatus>,
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
        if !&args.is_off() {
            let ssids = wifi.visible_ssid().context("Getting visible SSIDs")?;
            debug!("Visible SSIDs {:#?}", ssids);
            let mut found_ssid = false;
            // Search for known wifi in visible ssids
            for l in status_dict.keys() {
                if let Location::Known(wifi_substring) = l {
                    if ssids.iter().any(|x| x.contains(wifi_substring)) {
                        debug!("known wifi '{}' detected", wifi_substring);
                        found_ssid = true;
                        if let Some(mmstatus) = status_dict.get(l) {
                            state
                                .update_status(l.clone(), Some(mmstatus), &cache)
                                .context("Updating status")?;
                            break;
                        } else {
                            bail!("Internal error {:?} not found in statusdict", &l);
                        }
                    }
                }
            }
            if !found_ssid {
                debug!("Unknown wifi");
                state
                    .update_status(Location::Unknown, None, &cache)
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
