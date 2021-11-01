#![doc = include_str!("../README.md")]
use ::lib::mattermost::MMStatus;
use anyhow::{bail, Context, Result};
use figment::{providers::Serialized, Figment};
use shell_words::split;
use std::collections::HashMap;
use std::process::Command;
use std::time;

use ::lib::config::{Args, WifiStatusConfig};
use ::lib::state::{Cache, Location, State};
use ::lib::wifiscan::{WiFi, WifiInterface};
use std::path::PathBuf;
use std::thread::sleep;
//use tracing::subscriber:: set_global_default;
use tracing::{debug, info};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, layer::SubscriberExt//, EnvFilter
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
        let params = split(command)?;
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
    // get config from OS specific file path
    let cfg: Args = confy::load("automattermostatus")?;
    // Merge config Default → Config File → command line args
    let args = Figment::from(Serialized::defaults(Args::default()))
        .merge(Serialized::defaults(cfg))
        .merge(Serialized::defaults(args))
        .extract()?;
    debug!("Merge config and parameters : {:#?}", args);

    // Compute token if needed
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
        // Search for known wifi in visible ssids
        for l in status_dict.keys() {
            if let Location::Known(wifi_substring) = l {
                if ssids.iter().any(|x| x.contains(wifi_substring)) {
                    debug!("known wifi '{}' detected", wifi_substring);
                    found_ssid = true;
                    if let Some(mmstatus) = status_dict.get(l) {
                        state.update_status(l.clone(), Some(mmstatus), &cache)?;
                        break;
                    } else {
                        bail!("Internal error {:?} not found in statusdict", &l);
                    }
                }
            }
        }
        if !found_ssid {
            debug!("Unknown wifi");
            state.update_status(Location::Unknown, None, &cache)?;
        }
        if let Some(0) = args.delay {
            break;
        } else {
            sleep(delay_duration);
        }
    }

    Ok(())
}
