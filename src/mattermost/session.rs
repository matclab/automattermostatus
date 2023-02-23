//! This module implement mattermost session management.
//!
//! A session may be created via login:
//! ```
//! # use httpmock::prelude::*;
//! # let server = MockServer::start();
//! # let server_mock = server.mock(|expect, resp_with| {
//! #    expect.method(POST).path("/api/v4/users/login").json_body(
//! #         serde_json::json!({"login_id":"username","password":"passwordtext"}
//! #         ),
//! #     );
//! #    resp_with
//! #        .status(200)
//! #        .header("content-type", "application/json")
//! #        .header("Token", "xyzxyz")
//! #        .json_body(serde_json::json!({"id":"user_id"}));
//! # });
//! use lib::{BaseSession, Session};
//! let session = Session::new(&server.url(""))
//!                   .with_credentials("username", "passwordtext").login()?;
//! let token = session.token;
//! # server_mock.assert();
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! Or via a private access token:
//!
//! ```
//! # use tracing_subscriber::prelude::*;
//! # use tracing_subscriber::{fmt, layer::SubscriberExt};
//! # let fmt_layer = fmt::layer().with_target(false);
//! # tracing_subscriber::registry()
//! #    .with(fmt_layer)
//! #    .init();
//! use lib::{BaseSession, Session};
//! # use httpmock::prelude::*;
//! # let server = MockServer::start();
//! # let login_mock = server.mock(|expect, resp_with| {
//! #     expect
//! #         .method(GET)
//! #         .header("Authorization", "Bearer sdqgserdfmkjqBXHZFH:qgjr")
//! #         .path("/api/v4/users/me");
//! #     resp_with
//! #         .status(200)
//! #         .header("content-type", "application/json")
//! #         .json_body(serde_json::json!({"id":"user_id"}));
//! # });
//! let mut session = Session::new(&server.url(""))
//!                   .with_token("sdqgserdfmkjqBXHZFH:qgjr").login()?;
//! let token = session.token;
//! # login_mock.assert();
//! # Ok::<(), anyhow::Error>(())
//! ```
//! Types sequence is either one of :
//! - Session → SessionWithToken → LoggedSession
//! - Session → SessionWithCredentials → LoggedSession

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::mem;
use tracing::debug;

/// Trait implementing function necessary to establish a session (getting a authenticating token).
pub trait BaseSession {
    /// Get session token
    fn token(&self) -> Result<&str>;

    /// Get session `base_uri`
    fn base_uri(&self) -> &str;

    /// Login to mattermost instance
    fn login(&mut self) -> Result<LoggedSession>;
}

/// Base Session without authentication management
pub struct Session {
    #[allow(rustdoc::bare_urls)]
    /// base URL of the mattermost server like https://mattermost.example.com
    base_uri: String,
}

/// Implement [Session] authenticated with a private access token.
pub struct SessionWithToken {
    #[allow(rustdoc::bare_urls)]
    /// base URL of the mattermost server like https://mattermost.example.com
    pub base_uri: String,
    /// private access token for current user on the `base_uri` mattermost instance
    /// (either permanent and given at init or renewable with the help of login function)
    token: String,
}
///
/// Implement a session authenticated with a login and password
pub struct SessionWithCredentials {
    #[allow(rustdoc::bare_urls)]
    /// base URL of the mattermost server like https://mattermost.example.com
    pub base_uri: String,
    /// private access token for current user on the `base_uri` mattermost instance
    /// (either permanent and given at init or renewable with the help of login function)
    token: Option<String>,
    /// user login
    user: String,
    /// user password
    password: String,
}

///  Session once logged
#[derive(Debug)]
pub struct LoggedSession {
    #[allow(rustdoc::bare_urls)]
    /// base URL of the mattermost server like https://mattermost.example.com
    pub base_uri: String,
    /// (either permanent and given at init or renewable with the help of login function)
    pub token: String,
    /// Mattermost internal user_id
    pub user_id: String,
    // Used to relog when logged out
    user: Option<String>,
    password: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct LoginData {
    login_id: String,
    password: String,
}

impl Session {
    /// Create new empty [Session] to the `base_uri` mattermost server
    pub fn new(base_uri: &str) -> Self {
        Session {
            base_uri: base_uri.into(),
        }
    }
    /// Add existing token to current [Session]
    pub fn with_token(&mut self, token: &str) -> SessionWithToken {
        SessionWithToken {
            token: token.into(),
            base_uri: mem::take(&mut self.base_uri),
        }
    }
    /// Add login credentials to current [Session]
    pub fn with_credentials(&mut self, user_login: &str, password: &str) -> SessionWithCredentials {
        SessionWithCredentials {
            user: user_login.into(),
            password: password.into(),
            token: None,
            base_uri: mem::take(&mut self.base_uri),
        }
    }
}

impl BaseSession for SessionWithToken {
    fn token(&self) -> Result<&str> {
        Ok(&self.token)
    }
    fn base_uri(&self) -> &str {
        &self.base_uri
    }
    fn login(&mut self) -> Result<LoggedSession> {
        let uri = self.base_uri.to_owned() + "/api/v4/users/me";
        let json: serde_json::Value = ureq::get(&uri)
            .set("Authorization", &("Bearer ".to_owned() + &self.token))
            .call()?
            .into_json()?;
        debug!("User info: {:?}", json);
        Ok(LoggedSession {
            base_uri: mem::take(&mut self.base_uri),
            token: mem::take(&mut self.token),
            user_id: json["id"]
                .as_str()
                .ok_or(anyhow!("Received id is not a string"))?
                .to_string(),
            user: None,
            password: None,
        })
    }
}

impl BaseSession for SessionWithCredentials {
    fn token(&self) -> Result<&str> {
        if let Some(token) = &self.token {
            Ok(token)
        } else {
            Err(anyhow!("No token available, did login succeed ?"))
        }
    }
    fn base_uri(&self) -> &str {
        &self.base_uri
    }

    fn login(&mut self) -> Result<LoggedSession> {
        let uri = self.base_uri.to_owned() + "/api/v4/users/login";
        let response = ureq::post(&uri).send_json(serde_json::to_value(LoginData {
            login_id: self.user.clone(),
            password: self.password.clone(),
        })?)?;
        let Some(token) = response.header("Token") else {
            return Err(anyhow!(
                "Login authentication failed"
            ));
        };
        let token = token.to_string();
        let json: serde_json::Value = response.into_json()?;
        let user_id = json["id"].to_string();
        Ok(LoggedSession {
            base_uri: mem::take(&mut self.base_uri),
            token: token.to_string(),
            user_id,
            user: Some(self.user.clone()),
            password: Some(self.password.clone()),
        })
    }
}

impl LoggedSession {
    /// relog in case of a short lived session token obtained wia login/password
    pub fn relogin(&mut self) -> Result<&mut LoggedSession> {
        let (Some(password),Some(user)) = (self.password.clone(), self.user.clone()) else {
            // No login/password, we bail out without doing anything.
            return Ok(self);
        };

        let uri = self.base_uri.to_owned() + "/api/v4/users/login";
        let response = ureq::post(&uri).send_json(serde_json::to_value(LoginData {
            login_id: user,
            password,
        })?)?;
        let Some(token) = response.header("Token") else {
            return Err(anyhow!(
                "Login authentication failed"
            ));
        };
        self.token = token.to_string();
        Ok(self)
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use httpmock::prelude::*;
    use test_log::test; // Automatically trace tests
    #[test]
    fn login_with_success() -> Result<()> {
        // Start a lightweight mock server.
        let server = MockServer::start();

        // Create mocks on the server.
        let server_mock = server.mock(|expect, resp_with| {
            expect.method(POST).path("/api/v4/users/login").json_body(
                serde_json::json!({"login_id":"username","password":"passwordtext"}
                ),
            );
            resp_with
                .status(200)
                .header("content-type", "application/json")
                .header("Token", "xyzxyz")
                .json_body(serde_json::json!({"id":"user_id"}));
        });

        let mut session =
            Session::new(&server.url("")).with_credentials("username", "passwordtext");

        let session = session.login()?;

        // Send an HTTP request to the mock server. This simulates your code.
        //let token = login(&server.url(""), Some("username"), Some("passwordtext"))?;

        // Ensure the specified mock was called exactly one time (or fail with a detailed error description).
        server_mock.assert();
        // Ensure the mock server did respond as specified.
        assert_eq!(session.token, "xyzxyz");
        assert_eq!(session.base_uri, server.url(""));
        Ok(())
    }
    #[test]
    fn return_token() -> Result<()> {
        let session = Session::new("https://mattermost.example.com").with_token("xyzxyz");
        assert_eq!(session.base_uri, "https://mattermost.example.com");
        assert_eq!(session.token()?, "xyzxyz");
        Ok(())
    }
}
