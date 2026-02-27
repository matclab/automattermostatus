//! A newtype that redacts secrets in `Debug` and `Display` output.
//!
//! By default, formatting a [`Secret`] prints `***` instead of the inner
//! value.  Call [`enable_expose`] once (typically when `--expose-secrets` is
//! passed on the CLI) to make all `Secret` values visible in logs.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag controlling whether secrets are shown in `Debug`/`Display`.
static EXPOSE_SECRETS: AtomicBool = AtomicBool::new(false);

/// Enable secret exposure for `Debug` and `Display` formatting.
///
/// This is a one-way switch: once enabled it cannot be turned off.
pub fn enable_expose() {
    EXPOSE_SECRETS.store(true, Ordering::Relaxed);
}

/// A wrapper around `String` that redacts its value in `Debug` and `Display`
/// unless [`enable_expose`] has been called.
///
/// Use [`Secret::expose`] when you need the raw value regardless of the flag
/// (e.g. to send it over the wire).
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    /// Create a new `Secret` from a `String`.
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Return the inner value, ignoring the global expose flag.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if EXPOSE_SECRETS.load(Ordering::Relaxed) {
            write!(f, "Secret({:?})", self.0)
        } else {
            write!(f, "Secret(***)")
        }
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if EXPOSE_SECRETS.load(Ordering::Relaxed) {
            write!(f, "{}", self.0)
        } else {
            write!(f, "***")
        }
    }
}

impl FromStr for Secret {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reset the global flag before each test group — tests in this module are
    // serialized by nextest so this is safe.
    fn reset_flag() {
        EXPOSE_SECRETS.store(false, Ordering::Relaxed);
    }

    #[test]
    fn debug_redacts_by_default() {
        reset_flag();
        let s = Secret::new("hunter2".into());
        assert_eq!(format!("{:?}", s), "Secret(***)");
    }

    #[test]
    fn display_redacts_by_default() {
        reset_flag();
        let s = Secret::new("hunter2".into());
        assert_eq!(format!("{}", s), "***");
    }

    #[test]
    fn debug_shows_value_when_exposed() {
        reset_flag();
        enable_expose();
        let s = Secret::new("hunter2".into());
        assert_eq!(format!("{:?}", s), "Secret(\"hunter2\")");
        reset_flag();
    }

    #[test]
    fn display_shows_value_when_exposed() {
        reset_flag();
        enable_expose();
        let s = Secret::new("hunter2".into());
        assert_eq!(format!("{}", s), "hunter2");
        reset_flag();
    }

    #[test]
    fn expose_always_returns_inner_value() {
        reset_flag();
        let s = Secret::new("hunter2".into());
        assert_eq!(s.expose(), "hunter2");
    }

    #[test]
    fn from_str_works() {
        let s: Secret = "my_token".parse().unwrap();
        assert_eq!(s.expose(), "my_token");
    }

    #[test]
    fn serde_round_trip() {
        reset_flag();
        let original = Secret::new("top_secret".into());
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"top_secret\"");
        let deserialized: Secret = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.expose(), "top_secret");
    }
}
