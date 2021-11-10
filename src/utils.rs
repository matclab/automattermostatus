//! Simple utilities functions
use chrono::{DateTime, Local};
use tracing::warn;

/// Parse a stringwith the expected format "hh:mm" and return a [DateTime<Local>]
/// for the current day at time "hh:mm"
///
/// If `mm` is not parsable we return a datetime set at `hh:00`.

pub fn parse_from_hmstr(time_str: &Option<String>) -> Option<DateTime<Local>> {
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
        let res = Local::now().date().and_hms(hh, mm, 0);
        Some(res)
    } else {
        None
    }
}

//#[cfg(test)]
//mod should {
//    use super::*;
//
//    #[test]
//    fn take_into_account_timezone() {
//    }
//}