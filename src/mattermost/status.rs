//! Module responsible for sending custom status change to mattermost.
use crate::mattermost::BaseSession;
use crate::utils::parse_from_hmstr;
use anyhow::Result;
use chrono::{DateTime, Local};
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::fmt;
use thiserror::Error;
use tracing::debug;

/// Implement errors specific to `MMStatus`
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum MMSError {
    #[error("Bad json data")]
    BadJSONData(#[from] serde_json::error::Error),
    #[error("HTTP request error")]
    HTTPRequestError(#[from] ureq::Error),
    #[error("Mattermost login error")]
    LoginError(#[from] anyhow::Error),
}

/// Custom struct to serialize the HTTP POST data into a json objecting using serde_json
/// For a description of these fields see the [MatterMost OpenApi sources](https://github.com/mattermost/mattermost-api-reference/blob/master/v4/source/status.yaml)
#[derive(Derivative, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[derivative(Debug)]
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
    pub expires_at: Option<DateTime<Local>>,
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
    pub fn new(text: String, emoji: String) -> MMStatus {
        MMStatus {
            text,
            emoji,
            duration: None,
            expires_at: None,
        }
    }
    /// Add expiration time with the format "hh:mm" to the mattermost custom status
    pub fn expires_at(&mut self, time_str: &Option<String>) {
        // do not set expiry time if set in the past
        if let Some(expiry) = parse_from_hmstr(time_str) {
            if Local::now() < expiry {
                self.expires_at = Some(expiry);
                self.duration = Some("date_and_time".to_owned());
            } else {
                debug!("now {:?} >= expiry {:?}", Local::now(), expiry);
            }
        }
        // let dt: NaiveDateTime = NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11);
    }
    /// This function is essentially used for debugging or testing
    pub fn to_json(&self) -> Result<String, MMSError> {
        json::to_string(&self).map_err(MMSError::BadJSONData)
    }

    /// Send self custom status once
    #[allow(clippy::borrowed_box)] // Box needed beacause we can get two different types.
    pub fn _send(&self, session: &Box<dyn BaseSession>) -> Result<ureq::Response, ureq::Error> {
        let token = session
            .token()
            .expect("Internal Error: token is unset in current session");
        let uri = session.base_uri().to_owned() + "/api/v4/users/me/status/custom";
        ureq::put(&uri)
            .set("Authorization", &("Bearer ".to_owned() + token))
            .send_json(serde_json::to_value(&self).unwrap_or_else(|e| {
                panic!(
                    "Serialization of MMStatus '{:?}' failed with {:?}",
                    &self, &e
                )
            }))
    }
    /// Send self custom status, trying to login once in case of 401 failure.
    pub fn send(&mut self, session: &mut Box<dyn BaseSession>) -> Result<ureq::Response, MMSError> {
        debug!("Post status: {}", self.to_owned().to_json()?);
        match self._send(session) {
            Ok(response) => Ok(response),
            Err(ureq::Error::Status(code, response)) => {
                /* the server returned an unexpected status
                code (such as 400, 500 etc) */
                if code == 401 {
                    // relogin and retry
                    session.login().map_err(MMSError::LoginError)?;
                    self._send(session)
                } else {
                    Err(ureq::Error::Status(code, response))
                }
            }
            Err(e) => Err(e),
        }
        .map_err(MMSError::HTTPRequestError)
    }
}

#[cfg(test)]
mod send_should {
    use super::*;
    use crate::mattermost::{BaseSession, Session};
    use httpmock::prelude::*;
    #[test]
    fn send_required_json() -> Result<()> {
        // Start a lightweight mock server.
        let server = MockServer::start();
        let mut mmstatus = MMStatus::new("text".into(), "emoji".into());

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
        let mut session: Box<dyn BaseSession> =
            Box::new(Session::new(&server.url("")).with_token("token"));
        let resp = mmstatus.send(&mut session)?;

        // Ensure the specified mock was called exactly one time (or fail with a detailed error description).
        server_mock.assert();
        // Ensure the mock server did respond as specified.
        assert_eq!(resp.status(), 200);
        Ok(())
    }
}
