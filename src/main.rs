use anyhow::{bail, Context, Result};
use mattermost::MMStatus;
use std::collections::HashMap;
use std::process::Command;
use std::time;
use structopt::clap::AppSettings;
use shell_words::split;

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

    /// Home emoji and status (separated by two columns)
    #[structopt(long, env, default_value = "house::Travail à domicile")]
    home_status: String,
    ///
    /// Work emoji and status (separated by two columns)
    #[structopt(long, env, default_value = "systerel::Travail sur site")]
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

fn get_cache(dir: Option<String>) -> Result<Cache> {
    let mut state_file_name: PathBuf;
    if let Some(ref state_dir) = dir {
        state_file_name = PathBuf::from(state_dir);
    } else {
        state_file_name = PathBuf::from(
            env::var("XDG_CACHE_HOME").context(
                "No state directory defined neither from --state-dir option nor XDG_CACHE_HOME variable")?);
    }

    state_file_name.push("automattermostatus.state");
    Ok(Cache::new(state_file_name))
}

fn setup_tracing(args: &Args) {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_new(args.verbose.get_level_filter().to_string()).unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

fn update_token(mut args:Args) -> Result<Args> {
    if let Some(command) = &args.mm_token_cmd {
        let params = split(&command)?;
        debug!("Running command {}", command);
        let output = Command::new(&params[0])
            .args(&params[1..])
            .output()
            .context(format!("Error when running {}", &command))?;
        let token = String::from_utf8_lossy(&output.stdout);
        if token.len() == 0 {
            bail!("command '{}' returns nothing", &command);
        }
        //debug!("setting token to {}", token);
        args.mm_token = Some(token.to_string());
    }
    Ok(args)
}

fn prepare_status(args:&Args) -> Result<HashMap<Location, MMStatus>> {
    let mut res = HashMap::new();
    let split : Vec<&str> = args.home_status.split("::").collect();
    if split.len() != 2 {
        bail!("Expect home_status argument to contain one and only one :: separator");
    }
    res.insert(Location::Home,
        MMStatus::new(split[1].to_owned(), split[0].to_owned(), args.mm_url.clone(), args.mm_token.clone().unwrap()));

    let split : Vec<&str> = args.work_status.split("::").collect();
    if split.len() != 2 {
        bail!("Expect work_status argument to contain one and only one :: separator");
    }
    res.insert(Location::Work,
        MMStatus::new(split[1].to_owned(), split[0].to_owned(), args.mm_url.clone(), args.mm_token.clone().unwrap()));
    Ok(res)
}

#[paw::main]
fn main(args: Args) -> Result<()> {

    setup_tracing(&args);
    let args = update_token(args)?;
    let cache = get_cache(args.state_dir.to_owned())?;
    let status_dict = prepare_status(&args)?;

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
                state.update_status(Location::Unknown, &status_dict, &cache)?;
            } else {
                state.update_status(Location::Work, &status_dict, &cache)?;
            }
        } else if ssids.iter().any(|x| x.contains(&args.home_ssid)) {
            debug!("Home wifi detected");
            state.update_status(Location::Home, & status_dict,  &cache)?;
        } else {
            debug!("Unknown wifi");
            state.update_status(Location::Unknown, &status_dict, &cache)?;
        }
        if args.delay == 0 {
            break;
        } else {
            sleep(delay_duration);
        }
    }

    Ok(())
}
