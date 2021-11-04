//! Module responsible for sending custom status change to mattermost.
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::fmt;
use thiserror::Error;
use tracing::{debug, log::warn};

/// Implement errors specific to `MMStatus`
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum MMSError {
    #[error("Bad json data")]
    BadJSONData(#[from] serde_json::error::Error),
    #[error("HTTP request error")]
    HTTPRequestError(#[from] ureq::Error),
}

/// Custom struct to serialize the HTTP POST data into a json objecting using serde_json
/// For a description of these fields see the [MatterMost OpenApi sources](https://github.com/mattermost/mattermost-api-reference/blob/master/v4/source/status.yaml)
#[derive(Serialize, Deserialize)]
pub struct MMStatus {
    /// custom status text description
    pub text: String,
    /// custom status emoji name
    pub emoji: String,
    /// custom status duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// custom status expiration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// base URL of the mattermost server like https://mattermost.example.com
    #[serde(skip_serializing)]
    base_uri: String,
    /// private access token for current user on the `base_uri` mattermost instance
    #[serde(skip_serializing)]
    token: String,
}

impl fmt::Display for MMStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{} (duration: {:?}, expire at: {:?})",
            self.emoji, self.text, self.duration, self.expires_at
        )
    }
}

impl MMStatus {
    /// Create a `MMStatus` ready to be sent to the `mm_base_uri` mattermost instance.
    /// Authentication is done with the private access `token`.
    pub fn new(text: String, emoji: String, mm_base_uri: String, token: String) -> MMStatus {
        let uri = mm_base_uri + "/api/v4/users/me/status/custom";
        MMStatus {
            text,
            emoji,
            duration: None,
            expires_at: None,
            base_uri: uri,
            token,
        }
    }
    /// Add expiration time with the format "hh:mm" to the mattermost custom status
    pub fn expires_at(mut self, time_str: &Option<String>) -> Self {
        if let Some(ref s) = time_str {
            let splitted: Vec<&str> = s.split(':').collect();
            let hh: u32 = match splitted[0].parse() {
                Ok(h) => h,
                Err(_) => {
                    warn!("Unable to get hour from {:?}", &time_str);
                    0
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
            let expiry = Utc::now().date().and_hms(hh, mm, 0);
            if Utc::now() < expiry {
                self.expires_at = Some(expiry);
                self.duration = Some("date_and_time".to_owned());
            } else {
                debug!("now {:?} >= expiry {:?}", Utc::now(), expiry);
            }
        }
        // let dt: NaiveDateTime = NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11);
        self
    }
    /// This function is essentially used for debugging or testing
    pub fn to_json(&self) -> Result<String, MMSError> {
        json::to_string(&self).map_err(MMSError::BadJSONData)
    }

    /// Send the new custom status
    pub fn send(&self) -> Result<u16, MMSError> {
        debug!("Post status: {}", self.to_owned().to_json()?);
        let response = ureq::put(&self.base_uri)
            .set("Authorization", &("Bearer ".to_owned() + &self.token))
            .send_json(serde_json::to_value(&self)?)
            .map_err(MMSError::HTTPRequestError)?;
        Ok(response.status())
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use httpmock::prelude::*;
    #[test]
    fn send_required_json_for_mmstatus() -> Result<()> {
        // Start a lightweight mock server.
        let server = MockServer::start();
        let mmstatus = MMStatus::new(
            "text".to_string(),
            "emoji".to_string(),
            server.url(""),
            "token".to_string(),
        );

        // Create a mock on the server.
        let server_mock = server.mock(|expect, resp_with| {
            expect
                .method(PUT)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me/status/custom")
                .json_body(serde_json::json!({"emoji":"emoji","text":"text"}
                ));
            resp_with
                .status(200)
                .header("content-type", "text/html")
                .body("ok");
        });

        // Send an HTTP request to the mock server. This simulates your code.
        let status = mmstatus.send()?;

        // Ensure the specified mock was called exactly one time (or fail with a detailed error description).
        server_mock.assert();
        // Ensure the mock server did respond as specified.
        assert_eq!(status, 200);
        Ok(())
    }
}
