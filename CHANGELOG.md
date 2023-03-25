# v0.2.2 

Add feature to set the status to *do not disturb* when the microphone is used
by configured applications (linux only for now).

Update dependencies.


# v0.2.1 

Tentative fix for #2 where mattermosr instance does not always take the 
custom status into account despite returning 200.

# v0.2.0 

## Breaking change
Configuration file has move. Please move it before upgrading.
- On windows configuration file is read from `{FOLDERID_RoamingAppData}\ams\automattermostatus\config` instead of `{FOLDERID_RoamingAppData}\clabaut\automattermostatus\config`.
- On MacOS  configuration file is read from `$HOME/Library/Application Support/net.ams.automattermost` instead of `$HOME/Library/Application Support/net.clabaut.automattermost`.

# v0.1.10 : Bug correction

## Bugs

- #2 which prevent a status update if there is some connection problem at
  startup

# v0.1.9 : debian package

No functional changes nor bug fixes.

# v0.1.8 : login simplification

## Feature

- possibility to use login+password to connect to mattermost server

## Bugs

- allow to pass `delay` of more than 255s on command line
- continue to match wifi event after finding an empty SSID

# v0.1.7 : use OS keyring and allow status for off time

- Lookup OS keyring for mattermost token.  Should work on all three supported OS.
- Use empty `wifi_substring` to define a status that will be used for off
  times.

# v0.1.6 : Correct typo in XDG desktop file

# v0.1.5 : Correct message expiry implementation

- expiry is now computed when sending message

# v0.1.4 : Implement Off time and message expiry

- add begin and end time parameters
- add off days configuration with week selection by parity
- add expires_at to define mattermosts status expiration time
- better error messages

# v0.1.3 : Still working release process

No functional change.

# v0.1.2 : Working on CI and release process
No functional change.

# v0.1.0 : Initial release
All basic functionalities :
- get visible SSID on three major OS,
- update mattermost custom status,
- use configuration file.
