//! Implement persistant state for current location
//!
//! The [`State`] also provide the [`State::update_status`] function used to propagate the custom status
//! state to the mattermost instance
use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use tracing::{debug, info};

use crate::mattermost::{LoggedSession, MMCustomStatus};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// If more than MAX_SECS_BEFORE_FORCE_UPDATE are elapsed, we forcibly update
/// mattermost custom status to the expected value even if there was no change in visible
/// wifi SSIDs.
const MAX_SECS_BEFORE_FORCE_UPDATE: u64 = 60 * 60;

/// Struct implementing a cache for the application state
#[derive(Debug)]
pub struct Cache {
    path: PathBuf,
}

impl Cache {
    /// Create a cache at location `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

/// Wifi locations
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub enum Location {
    /// Known location based on wifi ssid substring match
    Known(String),
    /// Unknown location
    Unknown,
}

/// State containing at least location info
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    location: Location,
    lastchange_timestamp: i64,
}

impl State {
    /// Build a state, either by reading current persisted state in `cache`
    /// or by creating an empty default one.
    pub fn new(cache: &Cache) -> Result<Self> {
        if let Ok(json) = &fs::read(&cache.path) {
            if let Ok(res) = serde_json::from_str::<State>(&String::from_utf8_lossy(json)) {
                debug!("Previous known location `{:?}`", res.location);
                return Ok(res);
            }
        }
        Ok(Self {
            location: Location::Unknown,
            lastchange_timestamp: 0,
        })
    }

    /// Update state with location and ensure persisting of state on disk
    pub fn set_location(&mut self, location: Location, cache: &Cache) -> Result<()> {
        info!("Set location to `{:?}`", location);
        self.location = location;
        self.lastchange_timestamp = Utc::now().timestamp();
        fs::write(
            &cache.path,
            serde_json::to_string(&self)
                .unwrap_or_else(|_| panic!("Serialization of State Failed :{:?}", &self)),
        )
        .with_context(|| format!("Writing to cache file {:?}", cache.path))?;
        Ok(())
    }

    /// Update mattermost status depending upon current state
    ///
    /// If `current_location` is Unknown, then nothing is changed.
    /// If `current_location` is still the same for more than `MAX_SECS_BEFORE_FORCE_UPDATE`
    /// then we force update the mattermost status in order to catch up with desynchronise state
    /// Else we update mattermost status to the one associated to `current_location`.
    pub fn update_status(
        &mut self,
        current_location: Location,
        status: Option<&mut MMCustomStatus>,
        session: &mut LoggedSession,
        cache: &Cache,
        delay_between_polling: u64,
    ) -> Result<()> {
        if current_location == Location::Unknown {
            return Ok(());
        } else if current_location == self.location {
            // Less than max seconds have elapsed.
            // No need to update MM status again
            let elapsed_sec: u64 = (Utc::now().timestamp() - self.lastchange_timestamp)
                .try_into()
                .unwrap();
            if delay_between_polling * 2 < elapsed_sec
                && elapsed_sec <= MAX_SECS_BEFORE_FORCE_UPDATE
            {
                debug!(
                    "No change for {}s : no update to mattermost status",
                    MAX_SECS_BEFORE_FORCE_UPDATE
                );
                return Ok(());
            }
        }
        // We update the status on MM
        status.unwrap().send(session)?;
        // We update the location (only if setting mattermost status succeed)
        self.set_location(current_location, cache)?;
        Ok(())
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use mktemp::Temp;
    use test_log::test; // Automatically trace tests
    #[test]
    fn remember_state() -> Result<()> {
        let temp = Temp::new_file().unwrap().to_path_buf();
        let cache = Cache::new(temp);
        let mut state = State::new(&cache)?;
        assert_eq!(state.location, Location::Unknown);
        state.set_location(Location::Known("abcd".to_string()), &cache)?;
        assert_eq!(state.location, Location::Known("abcd".to_string()));
        let mut state = State::new(&cache)?;
        assert_eq!(state.location, Location::Known("abcd".to_string()));
        state.set_location(Location::Known("work".to_string()), &cache)?;
        assert_eq!(state.location, Location::Known("work".to_string()));
        Ok(())
    }
}
