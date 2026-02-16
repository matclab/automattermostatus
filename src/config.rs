#![allow(missing_docs)]
//! This module holds struct and helpers for parameters and configuration
//!
use crate::command::{CommandRunner, SystemCommandRunner};
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

/// Implement [`std::str::FromStr`] for [`WifiStatusConfig`] which allows to call `parse` from a
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
/// [`structopt::StructOpt`] implementing the verbosity parameter
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
    /// Returns `true` when no `-v`/`-q` flag was passed on the CLI.
    ///
    /// Used by `skip_serializing_if` so that the default CLI state does not
    /// overwrite the config file value during Figment merge.
    pub fn is_default_from_cli(&self) -> bool {
        self.verbosity_level == 0 && self.quiet_level == 0
    }

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
/// It will then update your mattermost custom status according to the config file.
///
// `Args` is the **parsing boundary**: it handles CLI (structopt) and config file
// (serde/TOML) deserialization where most fields are `Option<T>`. After merging
// defaults, config file and CLI, call [`Args::validate()`] to produce an
// [`AppConfig`] with all required fields guaranteed present.
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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

    /// List of application watched for using the microphone
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[structopt(short, long, name = "app binary name")]
    pub mic_app_names: Vec<String>,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    #[serde(
        deserialize_with = "de_from_str",
        skip_serializing_if = "QuietVerbose::is_default_from_cli"
    )]
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
            state_dir: ProjectDirs::from("net", "ams", "automattermostatus")
                .map(|p| p.cache_dir().to_owned()),
            mm_user: None,
            keyring_service: None,
            mm_secret: None,
            mm_secret_cmd: None,
            secret_type: Some(SecretType::Password),
            mm_url: Some("https://mattermost.example.com".into()),
            mic_app_names: Vec::new(),
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
                    Local::now().naive_local() < begin // now is before begin, we are off
                } else {
                    false // now is after begin, we are on duty if not after end
                }
            || if let Some(end) = parse_from_hmstr(&self.end) {
                    Local::now().naive_local() > end // now is after end, we are off
                } else {
                    false // now is before end, we are on duty
                }
    }
}

/// Validated Mattermost connection configuration (no Optional fields).
#[derive(Debug)]
pub struct MattermostConfig {
    /// Mattermost server URL
    pub url: String,
    /// Optional user name for login
    pub user: Option<String>,
    /// Authentication type
    pub secret_type: SecretType,
    /// Authentication secret (password or token)
    pub secret: String,
}

/// Validated schedule configuration.
#[derive(Debug)]
pub struct ScheduleConfig {
    /// Beginning of status update window (hh:mm format)
    pub begin: Option<String>,
    /// End of status update window (hh:mm format)
    pub end: Option<String>,
    /// Expiration time for custom status (hh:mm format)
    pub expires_at: Option<String>,
    /// Delay between polling in seconds
    pub delay: u32,
    /// Days off
    pub offdays: OffDays,
}

impl Off for ScheduleConfig {
    fn is_off_time(&self) -> bool {
        self.offdays.is_off_time()
            || if let Some(begin) = parse_from_hmstr(&self.begin) {
                Local::now().naive_local() < begin
            } else {
                false
            }
            || if let Some(end) = parse_from_hmstr(&self.end) {
                Local::now().naive_local() > end
            } else {
                false
            }
    }
}

/// Validated wifi configuration.
#[derive(Debug)]
pub struct WifiConfig {
    /// Wifi interface name (guaranteed non-empty after validation)
    pub interface_name: String,
    /// Status configuration triplets
    pub statuses: Vec<String>,
}

/// Validated microphone configuration.
#[derive(Debug)]
pub struct MicConfig {
    /// Application names to watch for microphone usage
    pub app_names: Vec<String>,
}

/// Fully validated application configuration produced from [`Args::validate()`].
///
/// Unlike [`Args`] (which uses `Option<T>` for CLI/TOML parsing), `AppConfig` has
/// all required fields guaranteed to be present. This eliminates the need for
/// `unwrap()`/`expect()` calls in the main application loop.
#[derive(Debug)]
pub struct AppConfig {
    /// Mattermost connection settings
    pub mattermost: MattermostConfig,
    /// Schedule/timing settings
    pub schedule: ScheduleConfig,
    /// Wifi interface settings
    pub wifi: WifiConfig,
    /// Microphone monitoring settings
    pub mic: MicConfig,
    /// Directory for persisting state
    pub state_dir: PathBuf,
}

impl Args {
    /// Validate all required fields and produce an [`AppConfig`].
    ///
    /// This is the boundary between CLI/TOML parsing (where fields are Optional)
    /// and the application logic (where required fields are guaranteed present).
    pub fn validate(self) -> Result<AppConfig> {
        let url = self
            .mm_url
            .context("Mattermost URL (mm_url) is not defined")?;
        let secret_type = self
            .secret_type
            .context("Secret type (secret_type) is not defined")?;
        let secret = self
            .mm_secret
            .context("Secret (mm_secret) is not defined")?;
        let delay = self.delay.context("Delay is not defined")?;
        let interface_name = self
            .interface_name
            .context("Wifi interface name (interface_name) is not defined")?;
        let state_dir = self
            .state_dir
            .context("State directory (state_dir) is not defined")?;

        Ok(AppConfig {
            mattermost: MattermostConfig {
                url,
                user: self.mm_user,
                secret_type,
                secret,
            },
            schedule: ScheduleConfig {
                begin: self.begin,
                end: self.end,
                expires_at: self.expires_at,
                delay,
                offdays: self.offdays,
            },
            wifi: WifiConfig {
                interface_name,
                statuses: self.status,
            },
            mic: MicConfig {
                app_names: self.mic_app_names,
            },
            state_dir,
        })
    }

    /// Update `args.mm_secret`  with the one fetched from OS keyring
    ///
    pub fn update_secret_with_keyring(mut self) -> Result<Self> {
        if let Some(user) = &self.mm_user {
            if let Some(service) = &self.keyring_service {
                let keyring = keyring::Entry::new(service, user)?;
                let secret = keyring.get_password().with_context(|| {
                    format!("Querying OS keyring (user: {user}, service: {service})")
                })?;
                self.mm_secret = Some(secret);
            } else {
                warn!("User is defined for keyring lookup but service is not");
                info!("Skipping keyring lookup");
            }
        }
        Ok(self)
    }

    /// Update `args.mm_secret`  with the standard output of
    /// `args.mm_secret_cmd` if defined.
    ///
    /// If the secret is a password, `secret` will be updated later when login to the mattermost
    /// server
    pub fn update_secret_with_command(self) -> Result<Args> {
        self.update_secret_with_command_using(&SystemCommandRunner)
    }

    /// Update `args.mm_secret` with the standard output of `args.mm_secret_cmd`,
    /// using the provided [`CommandRunner`].
    pub fn update_secret_with_command_using(mut self, runner: &dyn CommandRunner) -> Result<Args> {
        if let Some(command) = &self.mm_secret_cmd {
            let params =
                shell_words::split(command).context("Splitting mm_token_cmd into shell words")?;
            debug!("Running command {}", command);
            let args: Vec<String> = params[1..].to_vec();
            let secret = runner
                .run(&params[0], args)
                .with_context(|| format!("Error when running {}", &command))?;
            if secret.is_empty() {
                bail!("command '{}' returns nothing", &command);
            }
            self.mm_secret = Some(secret);
        }
        Ok(self)
    }

    /// Merge with precedence default [`Args`], config file and command line parameters.
    pub fn merge_config_and_params(&self) -> Result<Args> {
        let project_dirs = ProjectDirs::from("net", "ams", "automattermostatus")
            .context("Unable to find a project dir")?;
        let conf_dir = project_dirs.config_dir().to_owned();
        fs::create_dir_all(&conf_dir)
            .with_context(|| format!("Creating conf dir {:?}", &conf_dir))?;
        let conf_file = conf_dir.join("automattermostatus.toml");
        if !conf_file.exists() {
            info!("Write {:?} default config file", &conf_file);
            fs::write(&conf_file, toml::to_string(&Args::default())?)
                .with_context(|| format!("Unable to write default config file {conf_file:?}"))?;
        }

        self.merge_with_config_file(&conf_file)
    }

    /// Pure merge logic: Default → Config File → CLI args.
    ///
    /// Separated from [`merge_config_and_params`] so that tests can call it
    /// directly with a temporary TOML file, without filesystem side-effects.
    fn merge_with_config_file(&self, conf_file: &std::path::Path) -> Result<Args> {
        debug!("default Args : {:#?}", Args::default());
        // Merge defaults with config file for debug logging (non-fatal)
        if let Ok(config_args) = Figment::from(Serialized::defaults(Args::default()))
            .merge(Toml::file(conf_file))
            .extract::<Args>()
        {
            debug!("config Args : {:#?}", config_args);
        }
        debug!("parameter Args : {:#?}", self);
        // Merge config Default → Config File → command line args
        let res = Figment::from(Serialized::defaults(Args::default()))
            .merge(Toml::file(conf_file))
            .merge(Serialized::defaults(self))
            .extract()
            .context("Merging configuration file and parameters")?;
        debug!("Merged config and parameters : {:#?}", res);
        Ok(res)
    }

    /// Build an `Args` that mimics structopt parsing with no CLI flags.
    ///
    /// All `Option<T>` fields are `None`, `Vec`s are empty, and `verbose`
    /// has (0,0) — the structopt `from_occurrences` default.
    #[cfg(test)]
    fn cli_no_flags() -> Self {
        Args {
            interface_name: None,
            status: Vec::new(),
            mm_url: None,
            mm_user: None,
            secret_type: None,
            keyring_service: None,
            mm_secret: None,
            mm_secret_cmd: None,
            state_dir: None,
            begin: None,
            end: None,
            expires_at: None,
            delay: None,
            mic_app_names: Vec::new(),
            verbose: QuietVerbose {
                verbosity_level: 0,
                quiet_level: 0,
            },
            offdays: OffDays::default(),
        }
    }
}

#[cfg(test)]
mod merge_should {
    use super::*;
    use mktemp::Temp;
    use test_log::test;

    /// Write a TOML string to a temporary file and return the temp handle.
    fn write_toml(content: &str) -> Temp {
        let tmp = Temp::new_file().expect("create temp file");
        fs::write(tmp.as_path(), content).expect("write temp TOML");
        tmp
    }

    #[test]
    fn use_verbose_from_config_file() -> Result<()> {
        let tmp = write_toml("verbose = \"debug\"\n");
        let cli = Args::cli_no_flags();
        let merged = cli.merge_with_config_file(tmp.as_path())?;
        assert_eq!(merged.verbose.get_level_filter(), "Debug");
        Ok(())
    }

    #[test]
    fn cli_verbose_overrides_config() -> Result<()> {
        let tmp = write_toml("verbose = \"debug\"\n");
        let mut cli = Args::cli_no_flags();
        cli.verbose = QuietVerbose {
            verbosity_level: 0,
            quiet_level: 1,
        };
        let merged = cli.merge_with_config_file(tmp.as_path())?;
        assert_eq!(merged.verbose.get_level_filter(), "Error");
        Ok(())
    }

    #[test]
    fn use_delay_from_config_file() -> Result<()> {
        let tmp = write_toml("delay = 120\n");
        let cli = Args::cli_no_flags();
        let merged = cli.merge_with_config_file(tmp.as_path())?;
        assert_eq!(merged.delay, Some(120));
        Ok(())
    }

    #[test]
    fn cli_overrides_config_delay() -> Result<()> {
        let tmp = write_toml("delay = 120\n");
        let mut cli = Args::cli_no_flags();
        cli.delay = Some(30);
        let merged = cli.merge_with_config_file(tmp.as_path())?;
        assert_eq!(merged.delay, Some(30));
        Ok(())
    }

    #[test]
    fn use_verbose_from_full_default_config_file() -> Result<()> {
        // Generate the same TOML the program writes as default, then
        // change verbose from "Info" to "Debug" — simulates a user edit.
        let default_toml = toml::to_string(&Args::default())?;
        let edited_toml = default_toml.replace("verbose = \"Info\"", "verbose = \"Debug\"");
        assert!(
            edited_toml.contains("verbose = \"Debug\""),
            "Default config must contain a verbose line to edit. Got:\n{default_toml}"
        );
        let tmp = write_toml(&edited_toml);
        let cli = Args::cli_no_flags();
        let merged = cli.merge_with_config_file(tmp.as_path())?;
        assert_eq!(merged.verbose.get_level_filter(), "Debug");
        Ok(())
    }
}
