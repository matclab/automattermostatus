/// This module olds struct and helpers for parameters and configuration
use ::structopt::clap::AppSettings;
use anyhow;
use anyhow::{bail, Result};
use directories_next::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use structopt;
use tracing::debug;

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

// Courtesy of structopt_flags crate
#[derive(structopt::StructOpt, Debug, Clone)]
pub struct QuietVerbose {
    /// Increase the output's verbosity level
    ///
    /// Pass many times to increase verbosity level, up to 3.
    #[structopt(
        name = "quietverbose",
        long = "verbose",
        short = "v",
        parse(from_occurrences),
        conflicts_with = "quietquiet",
        global = true
    )]
    verbosity_level: u8,

    /// Decrease the output's verbosity level.
    ///
    /// Used once, it will set error log level.
    /// Used twice, will silent the log completely
    #[structopt(
        name = "quietquiet",
        long = "quiet",
        short = "q",
        parse(from_occurrences),
        conflicts_with = "quietverbose",
        global = true
    )]
    quiet_level: u8,
}

impl Default for QuietVerbose {
    fn default() -> Self {
        QuietVerbose {
            verbosity_level: 1,
            quiet_level: 0,
        }
    }
}

impl Serialize for QuietVerbose {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.get_level_filter())
    }
}

fn de_from_str<'de, D>(deserializer: D) -> Result<QuietVerbose, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.to_ascii_lowercase().as_ref() {
        "off" => Ok(QuietVerbose {
            verbosity_level: 0,
            quiet_level: 2,
        }),
        "error" => Ok(QuietVerbose {
            verbosity_level: 0,
            quiet_level: 1,
        }),
        "warn" => Ok(QuietVerbose {
            verbosity_level: 0,
            quiet_level: 0,
        }),
        "info" => Ok(QuietVerbose {
            verbosity_level: 1,
            quiet_level: 0,
        }),
        "debug" => Ok(QuietVerbose {
            verbosity_level: 2,
            quiet_level: 0,
        }),
        _ => Ok(QuietVerbose {
            verbosity_level: 3,
            quiet_level: 0,
        }),
    }
}

impl QuietVerbose {
    pub fn get_level_filter(&self) -> &str {
        let quiet: i8 = if self.quiet_level > 1 {
            2
        } else {
            self.quiet_level as i8
        };
        let verbose: i8 = if self.verbosity_level > 2 {
            3
        } else {
            self.verbosity_level as i8
        };
        match verbose - quiet {
            -2 => "Off",
            -1 => "Error",
            0 => "Warn",
            1 => "Info",
            2 => "Debug",
            _ => "Trace",
        }
    }
}

#[derive(structopt::StructOpt, Serialize, Deserialize, Debug)]
/// Automate mattermost status with the help of wifi network
///
/// Use current available SSID of wifi networks to automate your mattermost status.
/// This program is mean to be call regularly and will update status according to the config file
#[structopt(global_settings(&[AppSettings::ColoredHelp, AppSettings::ColorAuto]))]
pub struct Args {
    /// wifi interface name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(short, long, env)]
    pub interface_name: Option<String>,

    /// Status configuration triplets (:: separated)
    ///
    /// Each triplet shall have the format:
    /// "wifi_substring::emoji_name::status_text"
    #[structopt(short, long)]
    pub status: Vec<String>,

    /// mattermost URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(short = "u", long, env)]
    pub mm_url: Option<String>,

    /// mattermost private Token
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, hide_env_values = true)]
    pub mm_token: Option<String>,

    /// mattermost private Token command
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env)]
    pub mm_token_cmd: Option<String>,

    /// directory for state file
    ///
    /// Will use content of XDG_CACHE_HOME if unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, parse(from_os_str))]
    pub state_dir: Option<PathBuf>,

    /// delay between wifi SSID polling in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env)]
    pub delay: Option<u8>,

    #[structopt(flatten)]
    #[serde(deserialize_with = "de_from_str")]
    pub verbose: QuietVerbose,
}

impl Default for Args {
    fn default() -> Args {
        let res = Args {
            #[cfg(target_os = "linux")]
            interface_name: Some("wlan0".into()),
            #[cfg(target_os = "windows")]
            interface_name: Some("Wireless Network Connection".into()),
            #[cfg(target_os = "macos")]
            interface_name: Some("en0".into()),
            status: [].to_vec(),
            delay: Some(60),
            state_dir: Some(
                ProjectDirs::from("net", "clabaut", "automattermostatus")
                    .unwrap()
                    .cache_dir()
                    .to_owned(),
            ),
            mm_token: None,
            mm_token_cmd: None,
            mm_url: Some("https://mattermost.com".into()),
            verbose: QuietVerbose {
                verbosity_level: 1,
                quiet_level: 0,
            },
        };
        debug!("Args::default : {:#?}", res);
        res
    }
}
