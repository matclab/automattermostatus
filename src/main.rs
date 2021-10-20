use anyhow::{bail, Context, Result};
use std::process::Command;
use std::time;
use structopt::clap::AppSettings;

mod mattermost;
mod platforms;
use platforms::{WiFi, WifiInterface};
use std::env;
use std::path::{Path, PathBuf};
use std::thread::sleep;
mod state;
use state::{Cache, Location, State};
//use tracing::subscriber:: set_global_default;
use structopt_flags::LogLevel;
use tracing::{debug, error, info, span, warn, Level};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry}; // to access get_log_level

#[derive(structopt::StructOpt)]
/// Automate mattermost status with the help of wifi network
///
/// Use current available SSID of wifi networks to automate your mattermost status.
/// This program is mean to be call regularly and will update status according to the config file
#[structopt(global_settings(&[AppSettings::ColoredHelp, AppSettings::ColorAuto]))]
struct Args {
    /// wifi interface name
    #[structopt(short, long, env, default_value = "wlan0")]
    interface_name: String,

    /// work SSID substring
    ///
    /// string that shall be contains in a visible SSID to be considered at work
    #[structopt(short = "W", long, env)]
    work_ssid: String,

    /// home SSID substring
    ///
    /// string that shall be contains in a visible SSID to be considered at home
    #[structopt(short = "H", long, env)]
    home_ssid: String,

    /// Home emoji and status (separated by column)
    #[structopt(long, env, default_value = "home:Travail à domicile")]
    home_status: String,
    ///
    /// Work emoji and status (separated by column)
    #[structopt(long, env, default_value = "systerel:Travail sur site")]
    work_status: String,

    /// mattermost URL
    #[structopt(short = "u", long, env)]
    mm_url: String,

    /// mattermost private Token
    #[structopt(long, env, hide_env_values = true)]
    mm_token: Option<String>,

    /// mattermost private Token command
    #[structopt(long, env)]
    mm_token_cmd: Option<String>,

    /// directory for state file
    ///
    /// Will use content of XDG_CACHE_HOME if unset.
    #[structopt(long, env)]
    state_dir: Option<String>,

    /// delay between wifi SSID polling in seconds
    #[structopt(long, env, default_value = "60")]
    delay: u8,

    #[structopt(flatten)]
    verbose: structopt_flags::QuietVerbose,
    // A level of verbosity, and can be used multiple times
    //#[structopt(short, long, parse(from_occurrences))]
    //verbose: i32,
}

fn get_cache(dir: Option<String>) -> Result<Cache<'static>> {
    let mut state_file_name: PathBuf;
    if let Some(ref state_dir) = dir {
        state_file_name = PathBuf::from(state_dir);
    } else {
        state_file_name = PathBuf::from(
            env::var("XDG_CACHE_HOME").context(
                "No state directory defined neither from --state-dir option nor XDG_CACHE_HOME variable")?);
    }

    state_file_name.push("automattermostatus.state");
    Ok(Cache::new(&Box::new(state_file_name)))
}

#[paw::main]
fn main(args: Args) -> Result<()> {
    // Configure tracing (logging)
    //tracing_subscriber::fmt::init();

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_new(args.verbose.get_level_filter().to_string()).unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let cache = get_cache(args.state_dir)?;

    let mut state = State::new(&cache)?;
    let delay_duration = time::Duration::new(args.delay.into(), 0);
    let wifi = WiFi::new(&args.interface_name);
    if !wifi.is_wifi_enabled()? {
        bail!("wifi is disabled");
    } else {
        info!("Wifi is enabled");
    }
    loop {
        let ssids = wifi.visible_ssid()?;
        debug!("Visible SSIDs {:#?}", ssids);
        if ssids.iter().any(|x| x.contains(&args.work_ssid)) {
            debug!("Work wifi detected");
            if ssids.iter().any(|x| x.contains(&args.home_ssid)) {
                warn!(
                    "Visible SSID contains both home `{}` and work `{}` wifi",
                    &args.home_ssid, &args.work_ssid,
                );
                state.set_location(Location::Unknown, &cache)?;
            } else {
                state.set_location(Location::Work, &cache)?;
            }
        } else if ssids.iter().any(|x| x.contains(&args.home_ssid)) {
            debug!("Home wifi detected");
            state.set_location(Location::Home, &cache)?;
        } else {
            debug!("Unknown wifi");
            state.set_location(Location::Unknown, &cache)?;
        }
        if args.delay == 0 {
            break;
        } else {
            sleep(delay_duration);
        }
    }

    Ok(())
}
