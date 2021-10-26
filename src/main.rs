#!doc[ = include_str!("README.md")]
use anyhow::{bail, Context, Result};
use ::lib::mattermost::MMStatus;
use shell_words::split;
use std::collections::HashMap;
use std::process::Command;
use std::time;
use figment::{Figment, providers::{Serialized}};

use ::lib::config::{Args,WifiStatusConfig};
use ::lib::platforms::{WiFi, WifiInterface};
use std::path:: PathBuf;
use std::thread::sleep;
use ::lib::state::{Cache, Location, State};
//use tracing::subscriber:: set_global_default;
use tracing::{debug, info};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter}; // to access get_log_level
use tracing_log::LogTracer;


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
fn setup_tracing(args: &Args) -> Result<()> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_new(args.verbose.get_level_filter().to_string()).unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
    LogTracer::init()?;
    Ok(())

}

/// Update `args.mm_token` with the standard output of
/// `args.mm_token_cmd` if defined.
fn update_token_with_command(mut args: Args) -> Result<Args> {
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
        let sc: WifiStatusConfig = s.parse()?;
        debug!("Adding : {:?}", sc);
        res.insert(
            Location::Known(sc.wifi_string),
            MMStatus::new(
                sc.text,
                sc.emoji,
                args.mm_url.as_ref().unwrap().clone(),
                args.mm_token.clone().unwrap(),
            ),
        );
    }
    Ok(res)
}

#[paw::main]
fn main(args: Args) -> Result<()> {
    setup_tracing(&args)?;
    let cfg : Args = confy::load("automattermostatus")?;
    let args = Figment::from(Serialized::defaults(Args::default()))
    .merge(Serialized::defaults(args))
    .merge(Serialized::defaults(cfg)).extract()?;
    debug!("Args : {:#?}", args);
    let args = update_token_with_command(args)?;
    let cache = get_cache(args.state_dir.to_owned())?;
    let status_dict = prepare_status(&args)?;

    let mut state = State::new(&cache)?;
    let delay_duration = time::Duration::new(args.delay.unwrap().into(), 0);
    let wifi = WiFi::new(&args.interface_name.unwrap());
    if !wifi.is_wifi_enabled()? {
        bail!("wifi is disabled");
    } else {
        info!("Wifi is enabled");
    }

    loop {
        let ssids = wifi.visible_ssid()?;
        debug!("Visible SSIDs {:#?}", ssids);
        let mut found_ssid = false;
        for l in status_dict.keys() {
            if let Location::Known(wifi_substring) = l {
                if ssids.iter().any(|x| x.contains(wifi_substring)) {
                    debug!("{} wifi detected", wifi_substring);
                    found_ssid = true;
                    let loc  = l.clone();
                    state.update_status(loc, &status_dict, &cache)?;
                }
            }
        }
        if !found_ssid {
            debug!("Unknown wifi");
            state.update_status(Location::Unknown, &status_dict, &cache)?;
        }
        if let Some(0) = args.delay {
            break;
        } else {
            sleep(delay_duration);
        }
    }

    Ok(())
}
