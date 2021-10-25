/// This module olds struct and helpers for parameters and configuration
use ::structopt::clap::AppSettings;
use anyhow::{bail, Result};
use structopt_flags;
use anyhow;
use structopt;


/// Object olding the configuration describing status that shall be send when 
/// a wifi with `wifi_string` is being seen.
#[derive(Debug, PartialEq)]
pub struct WifiStatusConfig {
    pub wifi_string: String,
    pub emoji: String,
    pub text: String,
}

/// Implement FromStr for WifiStatusConfig which allows to call `parse` from a
/// parameter:
/// ```
/// use lib::config::WifiStatusConfig;
/// let wsc : WifiStatusConfig = "wifinet::house::Working home".parse().unwrap();
/// assert_eq!(wsc, WifiStatusConfig {
///                     wifi_string: "wifinet".to_owned(),
///                     emoji:"house".to_owned(),
///                     text: "Working home".to_owned() });
/// ```
impl std::str::FromStr for WifiStatusConfig {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split("::").collect();
        if splitted.len() != 3 {
            bail!(
                "Expect status argument to contain two and only two :: separator (in '{}')",
                &s
            );
        }
        Ok(WifiStatusConfig {
            wifi_string: splitted[0].to_owned(),
            emoji: splitted[1].to_owned(),
            text: splitted[2].to_owned(),
        })
    }
}

/// Implement FromStr for WifiStatusConfig.
/// Only use for StructOpt `default_value`
pub struct Status(Vec<WifiStatusConfig>);
impl std::str::FromStr for Status {
    type Err = anyhow::Error;
    // Convert "[ aaa::bbb:ccc， ddd:eee:fff ]" in Status struct
    // Only used for default parameter oo structopt status field
    // Note the use of a special UTF8 FullWidth comma :，
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().next() != Some('[') {
            bail!("Expect '[' as first char (in '{}')", &s);
        }
        if s.chars().last() != Some(']') {
            bail!("Expect ']' as last char (in '{}')", &s);
        }
        match s
            .split("，")
            .map(|x| x.trim().parse())
            .collect::<Result<Vec<WifiStatusConfig>, _>>()
        {
            Ok(v) => Ok(Status(v)),
            Err(v) => Err(v),
        }
    }
}

#[derive(structopt::StructOpt)]
/// Automate mattermost status with the help of wifi network
///
/// Use current available SSID of wifi networks to automate your mattermost status.
/// This program is mean to be call regularly and will update status according to the config file
#[structopt(global_settings(&[AppSettings::ColoredHelp, AppSettings::ColorAuto]))]
pub struct Args {
    /// wifi interface name
    //const WINDOWS_INTERFACE: &'static str = "Wireless Network Connection";
    // en0 for mac
    #[structopt(short, long, env, default_value = "wlan0")]
    pub interface_name: String,


    /// Status configuration triplets (:: separated)
    ///
    /// Each triplet shall have the format:
    /// "wifi_substring::emoji_name::status_text"
    #[structopt(
        short,
        long,
        default_value = "[systerel::systerel::Travail sur site，clabautnet::house::Travail à domicile]"
    )]
    pub status: Vec<String>,

    /// mattermost URL
    #[structopt(short = "u", long, env)]
    pub mm_url: String,

    /// mattermost private Token
    #[structopt(long, env, hide_env_values = true)]
    pub mm_token: Option<String>,

    /// mattermost private Token command
    #[structopt(long, env)]
    pub mm_token_cmd: Option<String>,

    /// directory for state file
    ///
    /// Will use content of XDG_CACHE_HOME if unset.
    #[structopt(long, env)]
    pub state_dir: Option<String>,

    /// delay between wifi SSID polling in seconds
    #[structopt(long, env, default_value = "60")]
    pub delay: u8,

    #[structopt(flatten)]
    pub verbose: structopt_flags::QuietVerbose,
    // A level of verbosity, and can be used multiple times
    //#[structopt(short, long, parse(from_occurrences))]
    //verbose: i32,
}
