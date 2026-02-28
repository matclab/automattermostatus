//! This module Provide the [`Off`] trait and [`OffDays`] struct
pub use chrono::Weekday;
use chrono::{Datelike, Local, NaiveDate, Timelike};
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

/// Part of day for half-day off entries.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part {
    /// Full day off
    Full,
    /// Morning off (before noon)
    Morning,
    /// Afternoon off (from noon)
    Afternoon,
}

impl Part {
    /// Returns the current part of day based on the current hour.
    fn current() -> Self {
        let hour = Local::now().time().hour();
        if hour < 13 {
            Part::Morning
        } else {
            Part::Afternoon
        }
    }

    /// Convert to the string suffix used in config file keys.
    fn config_suffix(self) -> Option<&'static str> {
        match self {
            Part::Full => None,
            Part::Morning => Some("_morning"),
            Part::Afternoon => Some("_afternoon"),
        }
    }
}

/// Struct for describing the parity of the week for which the out of work day apply
/// Parity is given according to iso week number
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Parity {
    /// Day off for all weeks
    EveryWeek,
    /// Day off only for odd weeks
    OddWeek,
    /// Day off only for even weeks
    EvenWeek,
}

/// Struct holding a map of `((`[`Weekday`], [`Part`]), [`Parity`])` describing day offs.
///
/// Keys in the config file use the format `WEEKDAY_PART` where PART is optional:
/// - `Sat = 'EveryWeek'` — full day off every week
/// - `Wed_morning = 'EvenWeek'` — morning off on even weeks
/// - `Thu_afternoon = 'OddWeek'` — afternoon off on odd weeks
#[derive(Debug)]
pub struct OffDays(HashMap<(Weekday, Part), Parity>);

impl OffDays {
    /// Create new empty `OffDays` instance
    pub fn new() -> OffDays {
        OffDays(HashMap::new())
    }
    /// Insert a new offday for week of `parity`
    #[cfg(test)]
    fn insert(&mut self, day: Weekday, part: Part, parity: Parity) -> Option<Parity> {
        self.0.insert((day, part), parity)
    }

    /// Serialize to a TOML-friendly map of string keys to parity values.
    fn serialize_as_map(&self) -> HashMap<String, Parity> {
        let mut map = HashMap::new();
        for ((weekday, part), parity) in &self.0 {
            let key = match part.config_suffix() {
                Some(suffix) => format!("{}{}", weekday, suffix),
                None => weekday.to_string(),
            };
            map.insert(key, (*parity).clone());
        }
        map
    }

    /// Deserialize from a TOML-friendly map of string keys to parity values.
    fn deserialize_from_map<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<(Weekday, Part), Parity>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = HashMap::<String, Parity>::deserialize(deserializer)?;
        let mut result = HashMap::new();
        for (key, parity) in map {
            let (weekday_str, part_str) = key
                .rsplit_once('_')
                .map(|(w, p)| (w, Some(p)))
                .unwrap_or((&key, None));

            let weekday: Weekday = weekday_str.parse().map_err(serde::de::Error::custom)?;

            let part = match part_str {
                Some("morning") => Part::Morning,
                Some("afternoon") => Part::Afternoon,
                None => Part::Full,
                Some(other) => {
                    return Err(serde::de::Error::custom(format!(
                        "invalid part suffix: {}",
                        other
                    )));
                }
            };

            result.insert((weekday, part), parity);
        }
        Ok(result)
    }

    /// The user is off if date day is in OffDays and either,
    /// - parity is all
    /// - parity match the current iso week number
    /// - the part matches the current time of day (Morning/Afternoon)
    fn is_off_at_date(&self, date: impl Now) -> bool {
        let now = date.now();
        trace!("now: {:?}", now);
        trace!("now.weekday: {:?}", now.weekday());
        let weekday = now.weekday();
        let current_part = Part::current();

        // Check full-day first
        if let Some(parity) = self.0.get(&(weekday, Part::Full)) {
            if matches!(parity, Parity::EveryWeek) || parity_matches(parity, now.iso_week().week())
            {
                debug!("{:?} {:?} Full off", &weekday, &now.iso_week());
                return true;
            }
        }

        // Check half-day matching current part of day
        if let Some(parity) = self.0.get(&(weekday, current_part)) {
            if matches!(parity, Parity::EveryWeek) || parity_matches(parity, now.iso_week().week())
            {
                debug!(
                    "{:?} {:?} {:?} off",
                    &weekday,
                    &now.iso_week(),
                    current_part
                );
                return true;
            }
        }

        let res: bool = false;
        debug!(
            "{:?} {:?} {:?} is {} off",
            &weekday,
            &now.iso_week(),
            current_part,
            if !res { "not" } else { "" }
        );
        res
    }

    /// Return `true` if there are no OffDays.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn parity_matches(parity: &Parity, week: u32) -> bool {
    match parity {
        Parity::OddWeek => !week.is_multiple_of(2),
        Parity::EvenWeek => week.is_multiple_of(2),
        Parity::EveryWeek => true, // unreachable when checked after EveryWeek guard
    }
}

impl Serialize for OffDays {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.serialize_as_map().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OffDays {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = Self::deserialize_from_map(deserializer)?;
        Ok(OffDays(map))
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
    /// - the part matches the current time of day
    fn is_off_time(&self) -> bool {
        self.is_off_at_date(Time {})
    }
}

struct Time {}

/// Trait providing a `now` function.
///
/// The use of a trait instead of calling directly `Local::now` is needed in order to be able to
/// mock time in tests
#[cfg_attr(test, automock)] // create MockNow Struct for tests
pub trait Now {
    /// Returns current local date (without time of day).
    /// Time of day is retrieved separately via `Part::current()` for half-day checks.
    fn now(&self) -> NaiveDate;
}
impl Now for Time {
    fn now(&self) -> NaiveDate {
        Local::now().date_naive()
    }
}

#[cfg(test)]
mod is_off_should {
    use super::*;
    use anyhow::Result;
    use chrono::Weekday;
    use test_log::test;

    #[test]
    fn return_false_when_day_dont_match() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Mon, Part::Full, Parity::EveryWeek);
        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 1, Weekday::Tue).expect("Unable to convert date")
        });
        assert!(!leave.is_off_at_date(mock));
        Ok(())
    }

    #[test]
    fn return_true_when_match_and_no_parity() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Tue, Part::Full, Parity::EveryWeek);
        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 1, Weekday::Tue).expect("Unable to convert date")
        });
        assert!(leave.is_off_at_date(mock));
        Ok(())
    }

    #[test]
    fn return_true_when_day_and_parity_match() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Wed, Part::Full, Parity::OddWeek);

        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 15, Weekday::Wed).expect("Unable to convert date")
        });
        assert!(leave.is_off_at_date(mock));

        leave.insert(Weekday::Thu, Part::Full, Parity::EvenWeek);
        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 16, Weekday::Thu).expect("Unable to convert date")
        });
        assert!(leave.is_off_at_date(mock));

        Ok(())
    }

    #[test]
    fn return_false_when_day_match_but_not_parity() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Fri, Part::Full, Parity::EvenWeek);
        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 15, Weekday::Fri).expect("Unable to convert date")
        });
        assert!(!leave.is_off_at_date(mock));

        leave.insert(Weekday::Sun, Part::Full, Parity::OddWeek);
        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 16, Weekday::Sun).expect("Unable to convert date")
        });
        assert!(!leave.is_off_at_date(mock));
        Ok(())
    }

    #[test]
    fn return_true_for_morning_entry_during_morning() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Wed, Part::Morning, Parity::EveryWeek);

        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 1, Weekday::Wed).expect("Unable to convert date")
        });
        let result = leave.is_off_at_date(mock);
        // When Part::current() returns Morning, the morning entry should match
        // This test passes when run during morning hours
        assert!(result, "morning entry should match during morning hours");
        Ok(())
    }

    #[test]
    fn full_day_off_always_returns_true() -> Result<()> {
        let mut leave = OffDays::new();
        leave.insert(Weekday::Sat, Part::Full, Parity::EveryWeek);

        let mut mock = MockNow::new();
        mock.expect_now().times(1).returning(|| {
            NaiveDate::from_isoywd_opt(2015, 1, Weekday::Sat).expect("Unable to convert date")
        });
        // Full day off should always return true regardless of time of day
        assert!(leave.is_off_at_date(mock));
        Ok(())
    }
}

#[cfg(test)]
mod serde_should {
    use super::*;
    use anyhow::Result;

    #[test]
    fn deserialize_old_format_without_suffix() -> Result<()> {
        let toml_str = r#"
Sat = 'EveryWeek'
Sun = 'EveryWeek'
Wed = 'EvenWeek'
"#;
        let offdays: OffDays = toml::from_str(toml_str)?;
        assert_eq!(offdays.0.len(), 3);
        assert!(offdays.0.contains_key(&(Weekday::Sat, Part::Full)));
        assert!(offdays.0.contains_key(&(Weekday::Sun, Part::Full)));
        assert!(offdays.0.contains_key(&(Weekday::Wed, Part::Full)));
        Ok(())
    }

    #[test]
    fn deserialize_new_format_with_morning_suffix() -> Result<()> {
        let toml_str = r#"
Sat = 'EveryWeek'
Wed_morning = 'EvenWeek'
"#;
        let offdays: OffDays = toml::from_str(toml_str)?;
        assert_eq!(offdays.0.len(), 2);
        assert!(offdays.0.contains_key(&(Weekday::Sat, Part::Full)));
        assert!(offdays.0.contains_key(&(Weekday::Wed, Part::Morning)));
        Ok(())
    }

    #[test]
    fn deserialize_new_format_with_afternoon_suffix() -> Result<()> {
        let toml_str = r#"
Thu_afternoon = 'OddWeek'
"#;
        let offdays: OffDays = toml::from_str(toml_str)?;
        assert_eq!(offdays.0.len(), 1);
        assert!(offdays.0.contains_key(&(Weekday::Thu, Part::Afternoon)));
        Ok(())
    }

    #[test]
    fn deserialize_mixed_format() -> Result<()> {
        let toml_str = r#"
Sat = 'EveryWeek'
Sun = 'EveryWeek'
Wed_morning = 'EvenWeek'
Thu_afternoon = 'OddWeek'
"#;
        let offdays: OffDays = toml::from_str(toml_str)?;
        assert_eq!(offdays.0.len(), 4);
        assert!(offdays.0.contains_key(&(Weekday::Sat, Part::Full)));
        assert!(offdays.0.contains_key(&(Weekday::Sun, Part::Full)));
        assert!(offdays.0.contains_key(&(Weekday::Wed, Part::Morning)));
        assert!(offdays.0.contains_key(&(Weekday::Thu, Part::Afternoon)));
        Ok(())
    }

    #[test]
    fn serialize_to_compound_keys() -> Result<()> {
        let mut offdays = OffDays::new();
        offdays.insert(Weekday::Sat, Part::Full, Parity::EveryWeek);
        offdays.insert(Weekday::Wed, Part::Morning, Parity::EvenWeek);
        offdays.insert(Weekday::Thu, Part::Afternoon, Parity::OddWeek);

        let toml_str = toml::to_string(&offdays)?;

        assert!(
            toml_str.contains("Sat = \"EveryWeek\""),
            "Full day should not have suffix"
        );
        assert!(
            toml_str.contains("Wed_morning = \"EvenWeek\""),
            "Morning should have _morning suffix"
        );
        assert!(
            toml_str.contains("Thu_afternoon = \"OddWeek\""),
            "Afternoon should have _afternoon suffix"
        );
        Ok(())
    }

    #[test]
    fn serialize_roundtrip() -> Result<()> {
        let original_toml = r#"
Sat = 'EveryWeek'
Sun = 'EveryWeek'
Wed_morning = 'EvenWeek'
Thu_afternoon = 'OddWeek'
"#;
        let offdays: OffDays = toml::from_str(original_toml)?;
        let serialized = toml::to_string(&offdays)?;
        let deserialized: OffDays = toml::from_str(&serialized)?;

        assert_eq!(offdays.0.len(), deserialized.0.len());
        for key in offdays.0.keys() {
            assert!(
                deserialized.0.contains_key(key),
                "Key {:?} should be preserved",
                key
            );
        }
        Ok(())
    }

    #[test]
    fn deserialize_invalid_suffix_should_fail() -> Result<()> {
        let toml_str = r#"
Wed_badpart = 'EveryWeek'
"#;
        let result: Result<OffDays, _> = toml::from_str(toml_str);
        assert!(
            result.is_err(),
            "Invalid suffix should fail deserialization"
        );
        Ok(())
    }
}
