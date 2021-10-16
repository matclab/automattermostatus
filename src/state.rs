use anyhow::{Result};
use std::fs;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct Cache {
    pub(crate) path: Path,
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


impl State {
    pub fn new(path: &dyn AsRef<Path>) -> Result<Self> {
        let res : State;
        if let Ok(json) = &fs::read(path.as_ref()) {
            res = serde_json::from_str(&String::from_utf8_lossy(json))?;
        } else {
            res = Self {
            location: Location::Unknown,
            };
        }
        Ok(res)
    }

    pub fn set_location(&mut self, location: Location, path: &dyn AsRef<Path>) {
        self.location = location;
        fs::write(path.as_ref(), serde_json::to_string(&self).unwrap());
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
            let mut state = State::new(&temp)?;
            assert_eq!(state.location, Location::Unknown);
            state.set_location(Location::Home, &temp);
            assert_eq!(state.location, Location::Home);
            let mut state = State::new(&temp)?;
            assert_eq!(state.location, Location::Home);
            state.set_location(Location::Work, &temp);
            assert_eq!(state.location, Location::Work);
            Ok(())
        }

    }
}
