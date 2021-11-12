# auto*mat-termo-st*atus
Automate your mattermost custom status with the help of visible Wi-Fi SSID.
Development site is hosted on [gitlab](https://gitlab.com/matclab/automattermostatus).

## Usage
Here after is the command line help.
<!-- `$ target/debug/automattermostatus --help` as text -->
```text
automattermostatus 0.1.6
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
    -b, --begin <begin>                        
            beginning of status update with the format hh:mm
            
            Before this time the status won't be updated [env: BEGIN=]
        --delay <delay>                        
            delay between wifi SSID polling in seconds [env: DELAY=]

    -e, --end <end>                            
            end of status update with the format hh:mm
            
            After this time the status won't be updated [env: END=]
        --expires-at <expires-at>              
            Expiration time with the format hh:mm
            
            This parameter is used to set the custom status expiration time Set to "0" to avoid setting expiration time
            [env: EXPIRES_AT=]
    -i, --interface-name <interface-name>      
            wifi interface name [env: INTERFACE_NAME=]

        --keyring-service <keyring-service>    
            Service name used for mattermost private token lookup in OS keyring [env: KEYRING_SERVICE=]

        --keyring-user <keyring-user>          
            User name used for mattermost private token lookup in OS keyring [env: KEYRING_USER=]

        --mm-token <mm-token>                  
            mattermost private Token
            
            Usage of this option may leak your personal token. It is recommended to use `mm_token_cmd` or `keyring_user`
            and `keyring_service`. [env: MM_TOKEN]
        --mm-token-cmd <mm-token-cmd>          
            mattermost private Token command [env: MM_TOKEN_CMD=]

    -u, --mm-url <mm-url>                      
            mattermost URL [env: MM_URL=]

        --state-dir <state-dir>                
            directory for state file
            
            Will use content of XDG_CACHE_HOME if unset. [env: STATE_DIR=]
    -s, --status <status>...                   
            Status configuration triplets (:: separated)
            
            Each triplet shall have the format: "wifi_substring::emoji_name::status_text"
```
## Configuration
*Automattermostatus* get configuration from both a config file and a command
line (the later override the former).

### Config File
The config file is created if it does not exist.  It is created or read in the following places depending on your OS:
-    the [XDG user directory](https://www.freedesktop.org/wiki/Software/xdg-user-dirs/) specifications on Linux (usually `~/.config/automattermostatus/automattermostatus.toml`),
-    the [Known Folder system](https://msdn.microsoft.com/en-us/library/windows/desktop/dd378457.aspx) on Windows (usually `{FOLDERID_RoamingAppData}\automattermostatus\config`),
-    the [Standard Directories](https://developer.apple.com/library/content/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html#//apple_ref/doc/uid/TP40010672-CH2-SW6) on macOS (usually `$HOME/Library/Application Support/automattermost`).

A sample config file is:

<!-- `$ cat config.toml.example` as toml -->
```toml
# Automattermostatus example configuration
#
# Wifi interface name. Use to check that wifi is enabled (Mac and Windows)
interface_name = 'wlp0s20f3'

# Status string containing 3 fields separated by `::`
#  - First field is the wifi substring that should be contained in a visible SSID
#    for this status to be set
#  - Second field is the emoji string for the custom status
#  - Third field is the description text foir the custom status
#
status = ["corporatewifi::corplogo::On premise work",
	  "homenet::house::Working home"]

# Base url of the mattermost instanbce
mm_url = 'https://mattermost.example.com'

# Level of verbosity among Off, Error, Warn, Info, Debug, Trace
verbose = 'Info'

# mattermost private access token. It is recommended to use `mm_token_cmd` or
# better the OS keyring with `keyring_user` and `keyring_service`.
# mm_token= 'cieVee1Ohgeixaevo0Oiquiu'

# Command that should be executed to get mattermost private access token (the
# token shall be printed on stdout). See
# https://docs.mattermost.com/integrations/cloud-personal-access-tokens.html#creating-a-personal-access-token.
# It is recommended to use the OS keyring with `keyring_user` and `keyring_service`.
# mm_token_cmd = "secret-tool lookup name automattermostatus"


# *user* and *service* name used to query OS keyring in order to retrieve your
# mattermost private access token.
keyring_user = myname
keyring_service = mattermost_token

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

### Mattermost private token
Your [private
token](https://docs.mattermost.com/integrations/cloud-personal-access-tokens.html#creating-a-personal-access-token)
is available under `Account Parameters > Security > Personal Access Token`.
You should avoid to use `mm_token` parameter as it may leak your token to
other people having access to your computer. It is recommended to use the
`mm_token_cmd` option or better your local OS keyring with `keyring_user` and
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
keyring set mattermost_token username
```
```toml
# use the following configuration
keyring_user = 'username'
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
cargo build --release
```
The binaries are then found in the `target/release` directory.

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
