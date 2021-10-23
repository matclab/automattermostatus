# auto*mat-termo-st*atus

## Usage
<!-- `$ target/debug/automattermostatus -h` as text -->
```text
automattermostatus 0.1.0
Automate mattermost status with the help of wifi network

Use current available SSID of wifi networks to automate your mattermost status. This program is mean to be call
regularly and will update status according to the config file

USAGE:
    automattermostatus [FLAGS] [OPTIONS] --home-ssid <home-ssid> --mm-url <mm-url> --work-ssid <work-ssid>

FLAGS:
    -h, --help       Prints help information
    -q, --quiet      Decrease the output's verbosity level Used once, it will set error log level. Used twice, will
                     slient the log completely
    -v, --verbose    Increase the output's verbosity level Pass many times to increase verbosity level, up to 3
    -V, --version    Prints version information

OPTIONS:
        --delay <delay>                      delay between wifi SSID polling in seconds [env: DELAY=]  [default: 60]
    -H, --home-ssid <home-ssid>              home SSID substring [env: HOME_SSID=]
        --home-status <home-status>          Home emoji and status (separated by two columns) [env: HOME_STATUS=]
                                             [default: house::Travail à domicile]
    -i, --interface-name <interface-name>    wifi interface name [env: INTERFACE_NAME=]  [default: wlan0]
        --mm-token <mm-token>                mattermost private Token [env: MM_TOKEN]
        --mm-token-cmd <mm-token-cmd>        mattermost private Token command [env: MM_TOKEN_CMD=]
    -u, --mm-url <mm-url>                    mattermost URL [env: MM_URL=]
        --state-dir <state-dir>              directory for state file [env: STATE_DIR=]
    -W, --work-ssid <work-ssid>              work SSID substring [env: WORK_SSID=]
        --work-status <work-status>          Work emoji and status (separated by two columns) [env: WORK_STATUS=]
                                             [default: systerel::Travail sur site]
```


## Installation
You can either compile yourself or download the latest binaries from the
[release page](https://gitlab.com/matclab/automattermostatus/-/releases).


## Compilation

You can build the `automattermostatus` binary with:
```
cargo build --release
```
The binaries are then found in the `target/release` directory.

# License

Licensed under Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE) or https://www.apache.org/licenses/LICENSE-2.0)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be licensed as above, without any additional terms or
conditions.
