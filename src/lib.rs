#![warn(missing_docs)]
//! Automattermostatus main components :
//! - `config`: allow to configure the application from file and command line,
//! - `mattermost`:  updating custom status on a mattermost instance,
//! - `state`: persistent application state (essentially the location),
//! - `wifiscan`: wifi scanning for linux, macos and windows
pub mod config;
pub mod mattermost;
pub mod offtime;
pub mod state;
pub mod wifiscan;
