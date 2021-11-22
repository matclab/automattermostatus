//! This module Provide the [`Off`] trait and [`OffDays`] struct
pub use chrono::Weekday;
use chrono::{Date, Datelike, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace};

#[cfg(test)]
use mockall::automock;

/// Manage the time where the application shall not update the status because the user
/// is not working
pub trait Off {
    /// Is the user off now ?
    fn is_off_time(&self) -> bool;
}

/// Struct for describing the parity of the week for which the out of work day apply
/// Parity is given according to iso week number
#[derive(Serialize, Deserialize, Debug)]
pub enum Parity {
    /// Day off for all weeks
    EveryWeek,
    /// Day off only for odd weeks
    OddWeek,
    /// Day off only for even weeks
    EvenWeek,
}

/// Struct olding a map of ([`Weekday`], [`Parity`]) descripting day offs.
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct OffDays(HashMap<Weekday, Parity>);

struct Time {}

/// Trait providing a `now` function.
///
/// The use of a trait instead of calling directly `Local::now` is needed in order to be able to
/// mock time in tests
#[cfg_attr(test, automock)] // create MockNow Struct for tests
pub trait Now {
    /// Returns current local time
    fn now(&self) -> Date<Local>;
}
impl Now for Time {
    fn now(&self) -> Date<Local> {
        Local::now().date()
    }
}

impl OffDays {
    /// Create new empty `OffDays` instance
    pub fn new() -> OffDays {
        OffDays(HashMap::new())
    }
    #[allow(dead_code)]
    /// Insert a new offday for week of `parity`
    fn insert(&mut self, day: Weekday, parity: Parity) -> Option<Parity> {
        self.0.insert(day, parity)
    }
    /// The user is off if date day is in OffDays and either,
    /// - parity is all
    /// - parity match the current iso week number
    fn is_off_at_date(&self, date: impl Now) -> bool {
        let now = date.now();
        trace!("now: {:?}", now);
        trace!("now.weekday: {:?}", now.weekday());
        let res: bool;
        if let Some(parity) = self.0.get(&now.weekday()) {
            trace!("match and parity = {:?}", parity);
            res = match parity {
                Parity::EveryWeek => true,
                Parity::OddWeek => &now.iso_week().week() % 2 == 1,
                Parity::EvenWeek => &now.iso_week().week() % 2 == 0,
            };
        } else {
            res = false;
        }
        debug!(
            "{:?} {:?} is {} off",
            &now.weekday(),
            &now.iso_week(),
            if !res { "not" } else { "" }
        );
        res
    }

    /// Return `true` if there are no OffDays.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for OffDays {
    fn default() -> Self {
        OffDays::new()
    }
}

impl Off for OffDays {
    /// The user is off if
    /// current day is in OffDays and either,
    /// - parity is all
    /// - parity match the current iso week number
    fn is_off_time(&self) -> bool {
        self.is_off_at_date(Time {})
    }
}

#[cfg(test)]
mod is_off_should {
    use super::*;
    use anyhow::Result;
    use chrono::{Local, TimeZone, Weekday};
    use test_env_log::test; // Automatically trace tests

    #[test]
    fn return_false_when_day_dont_match() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Mon, Parity::EveryWeek);
        let mut mock = MockNow::new();
        mock.expect_now()
            .times(1)
            .returning(|| Local.isoywd(2015, 1, Weekday::Tue));
        assert_eq!(leave.is_off_at_date(mock), false);
        Ok(())
    }

    #[test]
    fn return_true_when_match_and_no_parity() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Tue, Parity::EveryWeek);
        let mut mock = MockNow::new();
        mock.expect_now()
            .times(1)
            .returning(|| Local.isoywd(2015, 1, Weekday::Tue));
        assert_eq!(leave.is_off_at_date(mock), true);
        Ok(())
    }

    #[test]
    fn return_true_when_day_and_parity_match() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Wed, Parity::OddWeek);

        let mut mock = MockNow::new();
        mock.expect_now()
            .times(1)
            .returning(|| Local.isoywd(2015, 15, Weekday::Wed));
        assert_eq!(leave.is_off_at_date(mock), true);

        leave.insert(Weekday::Thu, Parity::EvenWeek);
        let mut mock = MockNow::new();
        mock.expect_now()
            .times(1)
            .returning(|| Local.isoywd(2015, 16, Weekday::Thu));
        assert_eq!(leave.is_off_at_date(mock), true);

        Ok(())
    }

    #[test]
    fn return_false_when_day_match_but_not_parity() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Fri, Parity::EvenWeek);
        let mut mock = MockNow::new();
        mock.expect_now()
            .times(1)
            .returning(|| Local.isoywd(2015, 15, Weekday::Fri));
        assert_eq!(leave.is_off_at_date(mock), false);

        leave.insert(Weekday::Sun, Parity::OddWeek);
        let mut mock = MockNow::new();
        mock.expect_now()
            .times(1)
            .returning(|| Local.isoywd(2015, 16, Weekday::Sun));
        assert_eq!(leave.is_off_at_date(mock), false);
        Ok(())
    }
}
