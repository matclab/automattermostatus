//! This module implement mattermost session management.
//!
//! A session may be created via login:
//! ```
//! # use httpmock::prelude::*;
//! # let server = MockServer::start();
//! #   let server_mock = server.mock(|expect, resp_with| {
//! #      expect.method(POST).path("//api/v4/users/login").json_body(
//! #           serde_json::json!({"login_id":"username","password":"passwordtext"}
//! #           ),
//! #       );
//! #      resp_with
//! #          .status(200)
//! #          .header("content-type", "application/json")
//! #          .header("Token", "xyzxyz");
//! # });
//! use lib::{Session,BaseSession};
//! let mut session = Session::new(&server.url("/"))
//!                   .with_credentials("username", "passwordtext");
//! session.login()?;
//! let token = session.token()?;
//! # server_mock.assert();
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! Or via a private access token:
//!
//! ```
//! use lib::{Session,BaseSession};
//! let mut session = Session::new("https://mattermost.example.com")
//!                   .with_token("sdqgserdfmkjqBXHZFH:qgjr");
//! let token = session.token()?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::mem;

/// Trait implementing function necessary to establish a session (getting a authenticating token).
pub trait BaseSession {
    /// Get session token
    fn token(&self) -> Result<&str>;

    /// Get session `base_uri`
    fn base_uri(&self) -> &str;

    /// Login to mattermost instance
    fn login(&mut self) -> Result<()>;
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
pub struct SessionWithLogin {
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
    pub fn with_credentials(&mut self, user_login: &str, password: &str) -> SessionWithLogin {
        SessionWithLogin {
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
    fn login(&mut self) -> Result<()> {
        Ok(())
    }
}

impl BaseSession for SessionWithLogin {
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

    fn login(&mut self) -> Result<()> {
        let uri = self.base_uri.to_owned() + "/api/v4/users/login";
        let response = ureq::post(&uri).send_json(serde_json::to_value(LoginData {
            login_id: self.user.clone(),
            password: self.password.clone(),
        })?)?;
        if let Some(token) = response.header("Token") {
            self.token = Some(token.into());
            Ok(())
        } else {
            Err(anyhow!(
                "Login authentication failed (response: {})",
                response.into_string()?
            ))
        }
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

        // Create a mock on the server.
        let server_mock = server.mock(|expect, resp_with| {
            expect.method(POST).path("/api/v4/users/login").json_body(
                serde_json::json!({"login_id":"username","password":"passwordtext"}
                ),
            );
            resp_with
                .status(200)
                .header("content-type", "application/json")
                .header("Token", "xyzxyz");
        });

        let mut session =
            Session::new(&server.url("")).with_credentials("username", "passwordtext");

        session.login()?;

        // Send an HTTP request to the mock server. This simulates your code.
        //let token = login(&server.url(""), Some("username"), Some("passwordtext"))?;

        // Ensure the specified mock was called exactly one time (or fail with a detailed error description).
        server_mock.assert();
        // Ensure the mock server did respond as specified.
        assert_eq!(session.token()?, "xyzxyz");
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
