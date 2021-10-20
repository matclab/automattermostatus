use anyhow::{Result};
use std::{fs, io};
use thiserror::Error;
use tracing::{debug, info};

use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct Cache<'a> {
    pub(crate) path: &'a Path,
}

impl <'a>  Cache<'a> {
    pub fn new(path: &'a dyn AsRef<Path>) -> Self {
        Self { path : path.as_ref()  }
    }
    
}

#[derive(Serialize, Deserialize, Debug, PartialEq )]
pub enum Location {
    Home,
    Work,
    Unknown
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
   pub location: Location,
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache IO Error")]
    IoError(#[from] io::Error),
}

impl State {
    pub fn new(cache: &Cache) -> Result<Self> {
        let res : State;
        if let Ok(json) = &fs::read(cache.path) {
            res = serde_json::from_str(&String::from_utf8_lossy(json))?;
        } else {
            res = Self {
            location: Location::Unknown,
            };
        }
        debug!("Previous known location `{:?}`", res.location);
        Ok(res)
    }

    pub fn set_location(&mut self, location: Location, cache: &Cache) -> Result<()> {
        info!("Set location to `{:?}`", location);
        self.location = location;
        fs::write(cache.path, serde_json::to_string(&self).unwrap())
            .map_err(|err| CacheError::IoError(err))?;
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
            state.set_location(Location::Home, &cache)?;
            assert_eq!(state.location, Location::Home);
            let mut state = State::new(&cache)?;
            assert_eq!(state.location, Location::Home);
            state.set_location(Location::Work, &cache)?;
            assert_eq!(state.location, Location::Work);
            Ok(())
        }

    }
}
