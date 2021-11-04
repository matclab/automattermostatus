#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
use ::lib::mattermost::MMStatus;
use anyhow::{bail, Context, Result};
use directories_next::ProjectDirs;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use std::process::Command;
use std::time;
use std::{collections::HashMap, fs};

use ::lib::config::{Args, WifiStatusConfig};
use ::lib::state::{Cache, Location, State};
use ::lib::wifiscan::{WiFi, WifiInterface};
use std::path::PathBuf;
use std::thread::sleep;
//use tracing::subscriber:: set_global_default;
use tracing::{debug, info};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt, //, EnvFilter
}; // to access get_log_level

/// Return a `Cache` used to persist state.
fn get_cache(dir: Option<PathBuf>) -> Result<Cache> {
    let mut state_file_name: PathBuf;
    if let Some(ref state_dir) = dir {
        state_file_name = PathBuf::from(state_dir);
    } else {
        bail!("Internal Error, no `state_dir` configured");
    }

    state_file_name.push("automattermostatus.state");
    Ok(Cache::new(state_file_name))
}

/// Setup logging to stdout
/// (Tracing is a bit more involving to set up but will provide much more feature if needed)
fn setup_tracing(_args: &Args) -> Result<()> {
    let fmt_layer = fmt::layer().with_target(false);
    //let filter_layer = EnvFilter::try_new(args.verbose.get_level_filter().to_string()).unwrap();

    tracing_subscriber::registry()
        //.with(filter_layer)
        .with(fmt_layer)
        .init();
    Ok(())
}

/// Update `args.mm_token` with the standard output of
/// `args.mm_token_cmd` if defined.
fn update_token_with_command(mut args: Args) -> Result<Args> {
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
        // Do not spit secret on stdout on released binary.
        //debug!("setting token to {}", token);
        args.mm_token = Some(token.to_string());
    }
    Ok(args)
}

/// Prepare a dictionnary of `Status` ready to be send to mattermost
/// server depending upon the location being found.
fn prepare_status(args: &Args) -> Result<HashMap<Location, MMStatus>> {
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

#[paw::main]
fn main(args: Args) -> Result<()> {
    setup_tracing(&args).context("Setting up tracing")?;
    let default_args = Args::default();
    debug!("default Args : {:#?}", default_args);
    let conf_dir = ProjectDirs::from("net", "clabaut", "automattermostatus")
        .expect("Unable to find a project dir")
        .config_dir()
        .to_owned();
    fs::create_dir_all(&conf_dir).with_context(|| format!("Creating conf dir {:?}", &conf_dir))?;
    let conf_file = conf_dir.join("automattermostatus.toml");

    let config_args: Args = Figment::from(Toml::file(&conf_file)).extract()?;
    debug!("config Args : {:#?}", config_args);
    debug!("parameter Args : {:#?}", args);
    // Merge config Default → Config File → command line args
    let args = Figment::from(Serialized::defaults(Args::default()))
        .merge(Toml::file(&conf_file))
        .merge(Serialized::defaults(args))
        .extract()
        .context("Merging configuration file and parameters")?;
    debug!("Merged config and parameters : {:#?}", args);

    // Compute token if needed
    let args = update_token_with_command(args).context("Get private token from mm_token_cmd")?;
    let cache = get_cache(args.state_dir.to_owned()).context("Reading cached state")?;
    let status_dict = prepare_status(&args).context("Building custom status messages")?;

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
        if let Some(0) = args.delay {
            break;
        } else {
            sleep(delay_duration);
        }
    }

    Ok(())
}
