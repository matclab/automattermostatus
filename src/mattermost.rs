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
    /// This function allows us to convert from the struct to a string of JSON which a web server
    /// will accept
    pub fn to_json(self: &Self) -> Result<String, MMRSError> {
        json::to_string(&self).map_err(MMRSError::BadJSONData)
    }

    pub fn send(self: &Self) -> Result<reqwest::StatusCode, MMRSError> {
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
