//! Abstraction over external command execution.
//!
//! [`CommandRunner`] allows swapping the real system command execution
//! ([`SystemCommandRunner`]) with a mock in tests. This is necessary because
//! the application calls platform-specific CLI tools (nmcli, airport, netsh,
//! ioreg) that are unavailable in CI or on other platforms. Injecting a
//! [`CommandRunner`] makes wifi scanning and microphone detection testable
//! without requiring the actual hardware or OS utilities.

use anyhow::{Context, Result};

/// Trait for running external commands and capturing their stdout.
#[cfg_attr(test, mockall::automock)]
pub trait CommandRunner: Send + Sync {
    /// Run `cmd` with the given `args` and return its stdout as a [`String`].
    fn run(&self, cmd: &str, args: Vec<String>) -> Result<String>;
}

/// Default implementation that delegates to [`std::process::Command`].
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, cmd: &str, args: Vec<String>) -> Result<String> {
        let output = std::process::Command::new(cmd)
            .args(&args)
            .output()
            .with_context(|| format!("Running {cmd}"))?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
