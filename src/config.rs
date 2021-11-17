#![allow(missing_docs)]
//! This module holds struct and helpers for parameters and configuration
//!
use crate::offtime::{Off, OffDays};
use crate::utils::parse_from_hmstr;
use ::structopt::clap::AppSettings;
use anyhow::{bail, Context, Result};
use chrono::Local;
use directories_next::ProjectDirs;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use structopt;
use structopt::clap::arg_enum;
use tracing::{debug, info, warn};

arg_enum! {
/// Enum used to encode `secret_type` parameter (password or token)
///
/// When set to [Password], the secret is used to obtain a session token
/// by using the login API. When set to [Token], the secret is a private access
/// token directly usable to access API.
#[derive(Serialize, Deserialize,Debug)]
pub enum SecretType {
    Token,
    Password,
}
}

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

/// Implement [`FromStr`] for [`WifiStatusConfig`] which allows to call `parse` from a
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
/// [`StructOpt`] implementing the verbosity parameter
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
    #[structopt(short, long, env, name = "itf_name")]
    pub interface_name: Option<String>,

    /// Status configuration triplets (:: separated)
    ///
    /// Each triplet shall have the format:
    /// "wifi_substring::emoji_name::status_text". If `wifi_substring` is empty, the ssociated
    /// status will be used for off time.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[structopt(short, long, name = "wifi_substr::emoji::text")]
    pub status: Vec<String>,

    /// mattermost URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(short = "u", long, env, name = "url")]
    pub mm_url: Option<String>,

    /// User name used for mattermost login or for password or private token lookup in OS keyring.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, name = "username")]
    pub mm_user: Option<String>,

    /// Type of secret. Either `Password` (default) or `Token`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(short = "t", long, env, possible_values = &SecretType::variants(), case_insensitive = true)]
    pub secret_type: Option<SecretType>,

    /// Service name used for mattermost secret lookup in OS keyring.
    ///
    /// The secret is either a `password` (default) or a`token` according to
    /// `secret_type` option
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, name = "token service name")]
    pub keyring_service: Option<String>,

    /// mattermost private Token
    ///
    /// Usage of this option may leak your personal token. It is recommended to
    /// use `mm_token_cmd` or `keyring_service`.
    ///
    /// The secret is either a `password` (default) or a`token` according to
    /// `secret_type` option
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, hide_env_values = true, name = "token")]
    pub mm_secret: Option<String>,

    /// mattermost secret command
    ///
    /// The secret is either a `password` (default) or a`token` according to
    /// `secret_type` option
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, name = "command")]
    pub mm_secret_cmd: Option<String>,

    /// directory for state file
    ///
    /// Will use content of XDG_CACHE_HOME if unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, parse(from_os_str), name = "cache dir")]
    pub state_dir: Option<PathBuf>,

    /// beginning of status update with the format hh:mm
    ///
    /// Before this time the status won't be updated
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(short, long, env, name = "begin hh:mm")]
    pub begin: Option<String>,

    /// end of status update with the format hh:mm
    ///
    /// After this time the status won't be updated
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(short, long, env, name = "end hh:mm")]
    pub end: Option<String>,

    /// Expiration time with the format hh:mm
    ///
    /// This parameter is used to set the custom status expiration time
    /// Set to "0" to avoid setting expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env, name = "expiry hh:mm")]
    pub expires_at: Option<String>,

    /// delay between wifi SSID polling in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    #[structopt(long, env)]
    pub delay: Option<u32>,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    #[serde(deserialize_with = "de_from_str")]
    pub verbose: QuietVerbose,

    #[structopt(skip)]
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
            status: ["home::house::working at home".to_string()].to_vec(),
            delay: Some(60),
            state_dir: Some(
                ProjectDirs::from("net", "clabaut", "automattermostatus")
                    .expect("Unable to find a project dir")
                    .cache_dir()
                    .to_owned(),
            ),
            mm_user: None,
            keyring_service: None,
            mm_secret: None,
            mm_secret_cmd: None,
            secret_type: Some(SecretType::Password),
            mm_url: Some("https://mattermost.example.com".into()),
            verbose: QuietVerbose {
                verbosity_level: 1,
                quiet_level: 0,
            },
            expires_at: Some("19:30".to_string()),
            begin: Some("8:00".to_string()),
            end: Some("19:30".to_string()),
            offdays: OffDays::default(),
        };
        res
    }
}

impl Off for Args {
    fn is_off_time(&self) -> bool {
        self.offdays.is_off_time() // The day is off, so we are off
            || if let Some(begin) = parse_from_hmstr(&self.begin) {
                    Local::now() < begin // now is before begin, we are off
                } else {
                    false // now is after begin, we are on duty if not after end
                }
            || if let Some(end) = parse_from_hmstr(&self.end) {
                    Local::now() > end // now is after end, we are off
                } else {
                    false // now is before end, we are on duty
                }
    }
}

impl Args {
    /// Update `args.mm_secret` and `args.token` with the one fetched from OS keyring
    ///
    /// If the secret is a password, [token] will be updated later when login to the mattermost
    /// server
    pub fn update_secret_with_keyring(mut self) -> Result<Self> {
        if let Some(user) = &self.mm_user {
            if let Some(service) = &self.keyring_service {
                let keyring = keyring::Keyring::new(service, user);
                let secret = keyring.get_password().with_context(|| {
                    format!("Querying OS keyring (user: {}, service: {})", user, service)
                })?;
                self.mm_secret = Some(secret);
            } else {
                warn!("User is defined for keyring lookup but service is not");
                info!("Skipping keyring lookup");
            }
        }
        Ok(self)
    }

    /// Update [self.mm_secret] and [self.token] with the standard output of
    /// [self.mm_secret_cmd] if defined.
    ///
    /// If the secret is a password, [token] will be updated later when login to the mattermost
    /// server
    pub fn update_secret_with_command(mut self) -> Result<Args> {
        if let Some(command) = &self.mm_secret_cmd {
            let params =
                shell_words::split(command).context("Splitting mm_token_cmd into shell words")?;
            debug!("Running command {}", command);
            let output = Command::new(&params[0])
                .args(&params[1..])
                .output()
                .context(format!("Error when running {}", &command))?;
            let secret = String::from_utf8_lossy(&output.stdout);
            if secret.len() == 0 {
                bail!("command '{}' returns nothing", &command);
            }
            // /!\ Do not spit secret on stdout on released binary.
            //debug!("setting secret to {}", secret);
            self.mm_secret = Some(secret.to_string());
        }
        Ok(self)
    }

    /// Merge with precedence default [`Args`], config file and command line parameters.
    pub fn merge_config_and_params(&self) -> Result<Args> {
        let default_args = Args::default();
        debug!("default Args : {:#?}", default_args);
        let conf_dir = ProjectDirs::from("net", "clabaut", "automattermostatus")
            .expect("Unable to find a project dir")
            .config_dir()
            .to_owned();
        fs::create_dir_all(&conf_dir)
            .with_context(|| format!("Creating conf dir {:?}", &conf_dir))?;
        let conf_file = conf_dir.join("automattermostatus.toml");
        if !conf_file.exists() {
            info!("Write {:?} default config file", &conf_file);
            fs::write(&conf_file, toml::to_string(&Args::default())?)
                .unwrap_or_else(|_| panic!("Unable to write default config file {:?}", conf_file));
        }

        let config_args: Args = Figment::from(Toml::file(&conf_file)).extract()?;
        debug!("config Args : {:#?}", config_args);
        debug!("parameter Args : {:#?}", self);
        // Merge config Default → Config File → command line args
        let res = Figment::from(Serialized::defaults(Args::default()))
            .merge(Toml::file(&conf_file))
            .merge(Serialized::defaults(self))
            .extract()
            .context("Merging configuration file and parameters")?;
        debug!("Merged config and parameters : {:#?}", res);
        Ok(res)
    }
}
