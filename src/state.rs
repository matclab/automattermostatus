use anyhow::{bail, Context, Result};
use chrono::{Utc};
use std::{collections::HashMap, fs, io};
use thiserror::Error;
use tracing::{debug, info};

use crate::mattermost::MMStatus;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_SECS_BEFORE_FORCE_UPDATE: i64 = 60 * 60;

pub struct Cache {
    pub(crate) path: PathBuf,
}

impl Cache {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub enum Location {
    Known(String),
    Unknown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    location: Location,
    timestamp: i64,
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache IO Error")]
    IoError(#[from] io::Error),
}

impl State {
    pub fn new(cache: &Cache) -> Result<Self> {
        let res: State;
        if let Ok(json) = &fs::read(&cache.path) {
            res = serde_json::from_str(&String::from_utf8_lossy(json)).context(format!(
                "Unable to deserialize state file {:?} (try to remove it)",
                &cache.path
            ))?;
        } else {
            res = Self {
                location: Location::Unknown,
                timestamp: 0,
            };
        }
        debug!("Previous known location `{:?}`", res.location);
        Ok(res)
    }

    pub fn set_location(&mut self, location: Location, cache: &Cache) -> Result<()> {
        info!("Set location to `{:?}`", location);
        self.location = location;
        self.timestamp = Utc::now().timestamp();
        fs::write(&cache.path, serde_json::to_string(&self).unwrap())
            .map_err(|err| CacheError::IoError(err))?;
        Ok(())
    }

    pub fn update_status(
        &mut self,
        location: Location,
        statusdict: &HashMap<Location, MMStatus>,
        cache: &Cache,
    ) -> Result<()> {
        if location == Location::Unknown {
            return Ok(());
        }
        else if location == self.location {
            // Less than max seconds have elapsed.
            // No need to update MM status again
            if Utc::now().timestamp() - self.timestamp <= MAX_SECS_BEFORE_FORCE_UPDATE {
                debug!(
                    "No change for {}s : no update to mattermost status",
                    MAX_SECS_BEFORE_FORCE_UPDATE
                );
                return Ok(());
            }
        }
        // We update the status on MM
        if let Some(mmstatus) = statusdict.get(&location) {
            self.set_location(location, cache)?;
            mmstatus.send()?;
        } else {
            bail!("Internal error {:?} not found in statusdict", location);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        use mktemp::Temp;
        #[test]
        fn remember_state() -> Result<()> {
            let temp = Temp::new_file().unwrap().to_path_buf();
            let cache = Cache::new(&temp);
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
}
