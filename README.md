---
lang: en
---
# auto*mat-termo-st*atus
Automate your mattermost custom status with the help of visible Wi-Fi SSID and
set your status to *do not disturb* when in visio (i.e. when a choosen
application is using your microphone — linux and windows only for now)

Development site is hosted on [gitlab](https://gitlab.com/matclab/automattermostatus).

Released binaries are available from [this
page](https://gitlab.com/matclab/automattermostatus/-/releases).

## Usage
Here after is the command line help.
<!-- `$ target/debug/automattermostatus --help` as text -->
```text
automattermostatus 0.2.2
Automate mattermost status with the help of wifi network

Use current visible wifi SSID to automate your mattermost status. This program is meant to either be running in
background or be call regularly with option `--delay 0`. It will then update your mattermost custom status according to
the config file

USAGE:
    automattermostatus [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       
            Prints help information

    -q, --quiet      
            Decrease the output's verbosity level.
            
            Used once, it will set error log level. Used twice, will silent the log completely
    -v, --verbose    
            Increase the output's verbosity level
            
            Pass many times to increase verbosity level, up to 3.
    -V, --version    
            Prints version information


OPTIONS:
    -m, --mic-app-names <app binary name>...      
            List of application watched for using the microphone

    -b, --begin <begin hh:mm>                     
            beginning of status update with the format hh:mm
            
            Before this time the status won't be updated [env: BEGIN=]
        --state-dir <cache dir>                   
            directory for state file
            
            Will use content of XDG_CACHE_HOME if unset. [env: STATE_DIR=]
        --mm-secret-cmd <command>                 
            mattermost secret command
            
            The secret is either a `password` (default) or a`token` according to `secret_type` option [env:
            MM_SECRET_CMD=]
        --delay <delay>                           
            delay between wifi SSID polling in seconds [env: DELAY=]

    -e, --end <end hh:mm>                         
            end of status update with the format hh:mm
            
            After this time the status won't be updated [env: END=]
        --expires-at <expiry hh:mm>               
            Expiration time with the format hh:mm
            
            This parameter is used to set the custom status expiration time Set to "0" to avoid setting expiration time
            [env: EXPIRES_AT=]
    -i, --interface-name <itf_name>               
            wifi interface name [env: INTERFACE_NAME=]

    -t, --secret-type <secret-type>
            Type of secret. Either `Password` (default) or `Token` [env: SECRET_TYPE=]  [possible values: Token,
            Password]
        --mm-secret <token>                       
            mattermost private Token
            
            Usage of this option may leak your personal token. It is recommended to use `mm_token_cmd` or
            `keyring_service`.
            
            The secret is either a `password` (default) or a`token` according to `secret_type` option [env: MM_SECRET]
        --keyring-service <token service name>
            Service name used for mattermost secret lookup in OS keyring.
            
            The secret is either a `password` (default) or a`token` according to `secret_type` option [env:
            KEYRING_SERVICE=]
    -u, --mm-url <url>                            
            mattermost URL [env: MM_URL=]

        --mm-user <username>
            User name used for mattermost login or for password or private token lookup in OS keyring [env: MM_USER=]

    -s, --status <wifi_substr::emoji::text>...    
            Status configuration triplets (:: separated)
            
            Each triplet shall have the format: "wifi_substring::emoji_name::status_text". If `wifi_substring` is empty,
            the ssociated status will be used for off time.
```
## Configuration
*Automattermostatus* get configuration from both a config file and a command
line (the later override the former).

### Config File
The config file is created if it does not exist.  It is created or read in the following places depending on your OS:
-    the [XDG user directory](https://www.freedesktop.org/wiki/Software/xdg-user-dirs/) specifications on Linux (usually `~/.config/automattermostatus/automattermostatus.toml`),
-    the [Known Folder system](https://msdn.microsoft.com/en-us/library/windows/desktop/dd378457.aspx) on Windows (usually `{FOLDERID_RoamingAppData}\ams\automattermostatus\config`),
-    the [Standard Directories](https://developer.apple.com/library/content/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html#//apple_ref/doc/uid/TP40010672-CH2-SW6) on macOS (usually `$HOME/Library/Application Support/net.ams.automattermost`).

A sample config file is:

<!-- `$ cat config.toml.example` as toml -->
```toml
# Automattermostatus example configuration
#
# Wifi interface name. Use to check that wifi is enabled (Mac and Windows)
interface_name = 'wlp0s20f3'

# Custom status string containing 3 fields separated by `::`
#  - First field is the wifi substring that should be contained in a visible SSID
#    for this status to be set. If empty the associated status wil be used for
#    off times.
#  - Second field is the emoji string for the custom status.
#  - Third field is the description text foir the custom status.
#
status = ["corporatewifi::corplogo::On premise work",
	  "homenet::house::Working home",
	  "::sleeping::Off time"]

# Base url of the mattermost instanbce
mm_url = 'https://mattermost.example.com'

# Mattermost staus will be set to *do not disturb* when one of those
# applications use the microphone.
mic_app_names = [ 'zoom', 'firefox', 'chromium' ]

# Level of verbosity among Off, Error, Warn, Info, Debug, Trace
verbose = 'Info'

# The type of the secret given by `mm_secret`, `mm_secret_cmd` or `kering_*`
# parameters. Either:
# secret_type = "Token" # for using a private acces token
# secret_type = "Password" # for using login and password credentials where
# the login is given by `mm_user`
secret_type = "Token"

# mattermost authentication secret. It is recommended to use `mm_secret_cmd` or
# better the OS keyring with `keyring_user` and `keyring_service`.
# mm_secret= 'cieVee1Ohgeixaevo0Oiquiu'

# Command that should be executed to get mattermost authentication secret (the
# secret shall be printed on stdout). See
# https://docs.mattermost.com/integrations/cloud-personal-access-secrets.html#creating-a-personal-access-secret.
# It is recommended to use the OS keyring with `keyring_service`.
# mm_secret_cmd = "secret-tool lookup name automattermostatus"


# *service* name used to query OS keyring in order to retrieve your
# mattermost private access secret. The user used to query the keyring is
# `mm_user`
keyring_service = 'mattermost_secret'

# set expiry time for custom mattermost status
expires_at = "19:30"

# set begin and end time of the working period. Outside of this period, custom
# status won't be set.
begin = "8:00"
end = "19:30"

# Definition of the day off (when automattermostatus do not update the user
# custom status). If a day is no present then it is considered as a workday.
# The attributes may be:
# - `EveryWeek`: the day is always off
# - `EvenWeek`: the day is off on even week (iso week number)
# - `OddWeek`: the day is off on odd week (iso week number)
[offdays]
Sat = 'EveryWeek'
Sun = 'EveryWeek'
Wed = 'EvenWeek'

```

### Mattermost Authentication Secret
The secret use to authenticate to the mattermost instance may be either a
*private access token* or a password associated with your username (see
`secret_type` configuration parameter).

The advantage of using your private access token is that it would work even if
you've set up a MFA (multi-factor authentication). The cons is that your
account shall have been explicitly authorized to use a *private access token*
by your mattermost instance administrator.

Your [private
token](https://docs.mattermost.com/integrations/cloud-personal-access-tokens.html#creating-a-personal-access-token), if enabled on your account,
is available under `Account Parameters > Security > Personal Access Token`.
You should avoid to use `mm_secret` parameter as it may leak your token to
other people having access to your computer. It is recommended to use the
`mm_secret_cmd` option or better your local OS keyring with `mm_user` and
`keyring_service` parameters. 

For example, on linux you may use `secret-tool`:
```sh
# store your token (it will ask you the token)
secret-tool store --label='token' name automattermostatus
# use the following command in `mm_token_cmd` to retrieve your token:
secret-tool lookup name automattermostatus
```
or the `keyring` command:
```sh
# store your token (it will ask you the token)
keyring set mattermost_token mattermost_username
```
```toml
# use the following configuration
secret_type = "Token"
mm_user = 'username'
keyring_service = 'mattermost_token'
```
On Mac OS you may use
[Keychain](https://en.wikipedia.org/wiki/Keychain_%28software%29) to store the
mattermost access token, and it will be looked up by *automattermostatus* with
a configuration similar to the one given here before.

On Windows, I have no mean to test, but it looks like you may use any software
based upon
[Microsoft Credential Locker](https://docs.microsoft.com/en-us/windows/uwp/security/credential-locker) to store your mattermost access token.


## Dependencies
On linux *automattermostatus* depends upon `NetworkManager` for getting the
visible SSIDs without root rights.

## Installation
You can either compile yourself, download the latest binaries from the
[release page](https://gitlab.com/matclab/automattermostatus/-/releases) or
install one of the available packages.

### Arch linux
Use your favorite aur helper. For example:
```
yay -S automattermostatus
```


## Compilation

You can build the `automattermostatus` binary with:
```
cargo build --release --locked
```
The binaries are then found in the `target/release` directory.

## Launch at Startup
### Linux
You may either copy the `distrib/automattermostatus.desktop` in
`/etc/xdg/autostart` or in `$HOME/.config/autostart` or if you use systemd,
you may copy the *auttoolmostatus systemd unit*
`distrib/automattermostatus.service` in `$HOME/.config/systemd/user` and do 
```sh
systemctl status --user enable --now automattermostatus
```
The logs are then visible with 
```sh
journalctl --user -u automattermostatus
```

### Windows

To launch at start-up, you can copy/paste the binary in
`C:\Users[user_name]\AppData\Roaming\Microsoft\Windows\Start
Menu\Programs\Startup`, but a terminal will appear at every session startup.


### Mac OS

**TODO**

# License

Licensed under Apache License, Version 2.0 ([LICENSE-APACHE](https://www.apache.org/licenses/LICENSE-2.0)).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be licensed as above, without any additional terms or
conditions.

### Issues
You may open issues or feature requests on [the gitlab issue
page](https://gitlab.com/matclab/automattermostatus/-/issues).

### Patch or Features
You may [fork](https://gitlab.com/matclab/automattermostatus/-/forks/new) the
project on gitlab, develop your patch or feature on a new branch and submit a
new merge request after having push back to your forked repo.

Do not hesitate to open an issue beforehand to discuss the bug fix strategy or
to ask about the feature you imagine.
