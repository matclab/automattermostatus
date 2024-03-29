//! Simple utilities functions
use chrono::{Local, NaiveDateTime};
use tracing::warn;

/// Parse a string with the expected format "hh:mm" and return a [`NaiveDateTime`]
/// for the current day at time "hh:mm"
///
/// If `mm` is not parsable we return a datetime set at `hh:00`.

pub fn parse_from_hmstr(time_str: &Option<String>) -> Option<NaiveDateTime> {
    if let Some(ref s) = time_str {
        let splitted: Vec<&str> = s.split(':').collect();
        let hh: u32 = match splitted[0].parse() {
            Ok(h) => h,
            Err(_) => {
                warn!("Unable to get hour from {:?}", &time_str);
                return None;
            }
        };
        let mm = if splitted.len() < 2 {
            0
        } else {
            match splitted[1].parse() {
                Ok(m) => m,
                Err(_) => {
                    warn!("Unable to get minutes from {:?}", &time_str);
                    0
                }
            }
        };

        Local::now().date_naive().and_hms_opt(hh, mm, 0)
    } else {
        None
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use test_log::test; // Automatically trace tests

    #[test]
    fn return_none_if_unparsable() {
        assert_eq!(None, parse_from_hmstr(&None));
        assert_eq!(None, parse_from_hmstr(&Some("biii".to_string())));
        assert_eq!(None, parse_from_hmstr(&Some(":12:30".to_string())));
    }
    #[test]
    fn return_hour_if_mn_is_unparsable() {
        let expect = Local::now().date_naive().and_hms_opt(12, 00, 0);
        assert_eq!(expect, parse_from_hmstr(&Some("12:3O".to_string())));
        assert_eq!(expect, parse_from_hmstr(&Some("12".to_string())));
    }
    #[test]
    fn return_expected_date() {
        let expect = Local::now().date_naive().and_hms_opt(7, 1, 0);
        assert_eq!(expect, parse_from_hmstr(&Some("07:01".to_string())));
        assert_eq!(expect, parse_from_hmstr(&Some("7:1".to_string())));
        let expect = Local::now().date_naive().and_hms_opt(23, 39, 0);
        assert_eq!(expect, parse_from_hmstr(&Some("23:39".to_string())));
    }
}
