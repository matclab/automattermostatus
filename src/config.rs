//! This module holds struct and helpers for parameters and configuration
//!
use crate::offtime::OffDays;
use ::structopt::clap::AppSettings;
use anyhow;
use anyhow::{bail, Result};
use directories_next::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use structopt;

/// Status that shall be send when a wifi with `wifi_string` is being seen.
#[derive(Debug, PartialEq)]
pub struct WifiStatusConfig {
    /// wifi SSID substring associated to this object custom status
    pub wifi_string: String,
    /// string description of the emoji that will be set as a custom status (like `home` for
    /// `:home:` mattermost emoji.
    pub emoji: String,
    /// custom status text description
    pub text: String,
}

/// Implement FromStr for WifiStatusConfig which allows to call `parse` from a
/// string representation:
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

// Courtesy of structopt_flags crate
/// `StructOpt` implementing the verbosity parameter
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
    /// Returns the string associated to the current verbose level
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
/// Use current visible wifi SSID to automate your mattermost status.
/// This program is meant to either be running in background or be call regularly
/// with option `--delay 0`.
/// It will then update your mattermost custom status according to the config file
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[structopt(short, long)]
    pub status: Vec<String>,

    /// mattermost URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(short = "u", long, env)]
    pub mm_url: Option<String>,

    /// mattermost private Token
    ///
    /// Usage of this option may leak your personal token. It is recommended to
    /// use `mm_token_cmd`.
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

    #[allow(missing_docs)]
    #[structopt(flatten)]
    #[serde(deserialize_with = "de_from_str")]
    pub verbose: QuietVerbose,

    #[structopt(skip)]
    //#[serde(skip_serializing_if = "OffDays::is_empty")]
    /// Days off for which the custom status shall not be changed
    pub offdays: OffDays,
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
                    .expect("Unable to find a project dir")
                    .cache_dir()
                    .to_owned(),
            ),
            mm_token: None,
            mm_token_cmd: None,
            mm_url: Some("https://mattermost.example.com".into()),
            verbose: QuietVerbose {
                verbosity_level: 1,
                quiet_level: 0,
            },
            offdays: OffDays::default(),
        };
        res
    }
}
