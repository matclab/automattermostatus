/// Module responsible for sending custom status change to mattermost.
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::fmt;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum MMRSError {
    #[error("Bad json data")]
    BadJSONData(#[from] serde_json::error::Error),
    #[error("HTTP request error")]
    HTTPRequestError(#[from] reqwest::Error),
}

/// Custom struct to serialize the HTTP POST data into a json objecting using serde_json
/// For a description of these fields see the [MatterMost OpenApi sources](https://github.com/mattermost/mattermost-api-reference/blob/master/v4/source/status.yaml)
#[derive(Serialize, Deserialize)]
pub struct MMStatus {
    pub text: String,
    pub emoji: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    uri: String,
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
    pub fn new(text: String, emoji: String, mm_api_uri: String, token: String) -> MMStatus {
        let uri = mm_api_uri + "/api/v4/users/me/status/custom";
        MMStatus {
            text,
            emoji,
            duration: None,
            expires_at: None,
            uri,
            token,
        }
    }
    /// This function is essentially used for debugging as `reqwest` is able to do the
    /// serialization by itself.
    pub fn to_json(&self) -> Result<String, MMRSError> {
        json::to_string(&self).map_err(MMRSError::BadJSONData)
    }

    /// Send the new custom status
    pub fn send(&self) -> Result<reqwest::StatusCode, MMRSError> {
        debug!("Post status: {}", self.to_owned().to_json()?);
        let status_code: reqwest::StatusCode = reqwest::blocking::Client::new()
            .put(&self.uri)
            .header("Authorization", "Bearer ".to_owned() + &self.token)
            .json(&self)
            .send()
            .map_err(MMRSError::HTTPRequestError)?
            .status();

        Ok(status_code)
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use httpmock::prelude::*;
    #[test]
    fn mmstatus_send_required_json() -> Result<()> {
        // Start a lightweight mock server.
        let server = MockServer::start();
        let mmstatus = MMStatus::new("text".to_string(), "emoji".to_string() ,server.url(""),"token".to_string());

        // Create a mock on the server.
        let server_mock = server.mock(|expect, resp_with| {
            expect.method(PUT)
                .header("Authorization", "Bearer token")
                .path("/api/v4/users/me/status/custom")
                .json_body(serde_json::json!({"emoji":"emoji","text":"text"}
));
            resp_with.status(200)
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
