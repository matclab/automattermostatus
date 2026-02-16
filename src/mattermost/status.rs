//! Module responsible for sending custom status change to mattermost.
use crate::mattermost::LoggedSession;
use crate::utils::parse_from_hmstr;
use anyhow::Result;
use chrono::{DateTime, Local, TimeZone};
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::fmt;
use thiserror::Error;
use tracing::{debug, error};

/// Implement errors specific to `MMCustomStatus`
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

type ResponseStatus = u16;

trait MMSendable {
    fn _send_at_once(
        &self,
        session: &LoggedSession,
        api_path: &str,
    ) -> Result<ResponseStatus, ureq::Error>;
    fn send_at(
        &mut self,
        session: &mut LoggedSession,
        api_path: &str,
    ) -> Result<ResponseStatus, MMSError>;
    fn to_json(&self) -> Result<String, MMSError>;
    #[allow(unused)]
    fn set_user_id(&mut self, user_id: String) {
        // empty implementation. Would be overrided if needed
    }
}

impl<T> MMSendable for T
where
    T: Serialize + std::fmt::Debug + Clone,
{
    /// This function is essentially used for debugging or testing
    fn to_json(&self) -> Result<String, MMSError> {
        json::to_string(&self).map_err(MMSError::BadJSONData)
    }

    /// Send self once as json
    /// `api_path` looks like "/api/v4/users/me/status/custom"
    #[allow(clippy::borrowed_box)] // Box needed beacause we can get two different types.
    fn _send_at_once(
        &self,
        session: &LoggedSession,
        api_path: &str,
    ) -> Result<ResponseStatus, ureq::Error> {
        let token = session.token.clone();
        let uri = session.base_uri.to_owned() + api_path;
        debug!("Sending {:?} to {}", self, uri);
        let json_value = serde_json::to_value(self).map_err(|e| {
            ureq::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        let resp = session
            .agent
            .put(&uri)
            .header("Authorization", &("Bearer ".to_owned() + &token))
            .send_json(json_value)?;
        Ok(resp.status().as_u16())
    }

    /// Send self as json, trying to login once in case of 401 failure.
    /// `api_path` looks like "/api/v4/users/me/status/custom"
    fn send_at(
        &mut self,
        session: &mut LoggedSession,
        api_path: &str,
    ) -> Result<ResponseStatus, MMSError> {
        debug!("Post status: {}", self.to_owned().to_json()?);
        match self._send_at_once(session, api_path) {
            Ok(status) => Ok(status),
            Err(ureq::Error::StatusCode(code)) => {
                /* the server returned an unexpected status
                code (such as 400, 500 etc) */
                if code == 401 {
                    // relogin and retry
                    let _ = session.relogin().map_err(MMSError::LoginError)?;
                    //self.set_user_id(loggedsession.user_id);
                    self._send_at_once(session, api_path)
                } else {
                    Err(ureq::Error::StatusCode(code))
                }
            }
            Err(e) => Err(e),
        }
        .map_err(MMSError::HTTPRequestError)
    }
}

/// Authorized status values for MM Status API
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Status {
    /// User is online
    #[serde(rename = "online")]
    Online,
    /// User is away
    #[serde(rename = "away")]
    Away,
    /// User is offline
    #[serde(rename = "offline")]
    Offline,
    /// User asked to not be disturbed
    #[serde(rename = "dnd")]
    Dnd,
}

/// Standard Mattermost status wire representation
#[derive(Derivative, Serialize, Deserialize, Clone)]
#[derivative(Debug)]
pub struct MMStatus {
    user_id: String,
    /// the requested status
    pub status: Status,
    dnd_end_time: i64,
}

impl MMStatus {
    /// Create a new status
    pub fn new(status: Status, user_id: String) -> MMStatus {
        MMStatus {
            user_id,
            status,
            dnd_end_time: Local::now().timestamp() + 300,
        }
    }

    /// set user_id
    pub fn set_user_id(&mut self, user_id: String) {
        self.user_id = user_id;
    }
    /// Send self as json, trying to login once in case of 401 failure.
    pub fn send(&mut self, session: &mut LoggedSession) {
        match self.send_at(session, "/api/v4/users/me/status") {
            Ok(_response) => (),
            Err(MMSError::HTTPRequestError(response)) => {
                /* the server returned an unexpected status
                code (such as 400, 500 etc) */
                error!("Unexpected response {:?}", response);
            }
            Err(_e) => (),
        };
    }
}

/// Custom struct to serialize the HTTP POST data into a json objecting using serde_json
/// For a description of these fields see the [MatterMost OpenApi sources](https://github.com/mattermost/mattermost-api-reference/blob/master/v4/source/status.yaml)
#[derive(Derivative, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[derivative(Debug)]
pub struct MMCustomStatus {
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

impl fmt::Display for MMCustomStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{} (duration: {:?}, expire at: {:?})",
            self.emoji, self.text, self.duration, self.expires_at
        )
    }
}

impl MMCustomStatus {
    /// Create a `MMCustomStatus` ready to be sent to the `mm_base_uri` mattermost instance.
    /// Authentication is done with the private access `token`.
    pub fn new(text: String, emoji: String) -> MMCustomStatus {
        MMCustomStatus {
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
            if Local::now().naive_local() < expiry {
                if let Some(local_dt) = Local.from_local_datetime(&expiry).latest() {
                    self.expires_at = Some(local_dt);
                    self.duration = Some("date_and_time".to_owned());
                }
            } else {
                debug!("now {:?} >= expiry {:?}", Local::now(), expiry);
            }
        }
        // let dt: NaiveDateTime = NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11);
    }
    /// Send self as json, trying to login once in case of 401 failure.
    pub fn send(&mut self, session: &mut LoggedSession) -> Result<ResponseStatus, MMSError> {
        self.send_at(session, "/api/v4/users/me/status/custom")
    }
}

#[cfg(test)]
mod send_should {
    use super::*;
    use crate::mattermost::{BaseSession, Session};
    use httpmock::prelude::*;
    use test_log::test; // Automatically trace tests
    #[test]
    fn send_required_json() -> Result<()> {
        // Start a lightweight mock server.
        let server = MockServer::start();
        let mut mmstatus = MMCustomStatus::new("text".into(), "emoji".into());

        // Create mocks on the server.
        let login_mock = server.mock(|expect, resp_with| {
            expect
                .method(GET)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me");
            resp_with
                .status(200)
                .header("content-type", "text/html")
                .json_body(serde_json::json!({"id":"user_id"}));
        });
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
        #[allow(unused_allocation)]
        let mut session = Box::new(Session::new(&server.url("")).with_token("token")).login()?;
        let resp = mmstatus.send(&mut session)?;

        // Ensure the specified mock was called exactly one time (or fail with a detailed error description).
        login_mock.assert();
        server_mock.assert();
        // Ensure the mock server did respond as specified.
        assert_eq!(resp, 200);
        Ok(())
    }
    #[test]
    fn catch_api_error() -> Result<()> {
        // Start a lightweight mock server.
        let server = MockServer::start();
        let mut mmstatus = MMCustomStatus::new("text".into(), "emoji".into());

        // Create mocks on the server.
        let login_mock = server.mock(|expect, resp_with| {
            expect
                .method(GET)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me");
            resp_with
                .status(200)
                .header("content-type", "text/html")
                .json_body(serde_json::json!({"id":"user_id"}));
        });
        let server_mock = server.mock(|expect, resp_with| {
            expect
                .method(PUT)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me/status/custom")
                .json_body(serde_json::json!({"emoji":"emoji","text":"text"}
                ));
            resp_with
                .status(500)
                .header("content-type", "text/html")
                .body("Internal error");
        });

        // Send an HTTP request to the mock server. This simulates your code.
        #[allow(unused_allocation)]
        let mut session = Box::new(Session::new(&server.url("")).with_token("token")).login()?;
        let resp = mmstatus.send(&mut session);
        assert!(resp.is_err());

        // Ensure the specified mock was called exactly one time (or fail with a detailed error description).
        login_mock.assert();
        server_mock.assert();
        Ok(())
    }
}
