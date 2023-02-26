//! Implement detection of process using microphone

use anyhow::Result;
use tracing::info;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::processes_owning_mic;

use crate::config::Args;
use crate::mattermost::{LoggedSession, MMStatus, Status};

/// Store MicUsage state
pub struct MicUsage {
    used: bool,
}

impl Default for MicUsage {
    fn default() -> Self {
        Self::new()
    }
}

impl MicUsage {
    /// Create new MicUsage struct
    pub fn new() -> Self {
        Self { used: false }
    }

    /// Update status to *do not disturb* if a known application use the mic
    pub fn update_dnd_status(
        &mut self,
        args: &Args,
        session: &mut LoggedSession,
    ) -> Result<&mut Self> {
        let names = processes_owning_mic()?;
        info!("Apps using mic: {:?}", names);
        let mut watched_app_found = false;
        for name in names {
            if args.mic_app_names.contains(&name) {
                watched_app_found = true;
                break;
            }
        }
        if watched_app_found && !self.used {
            let mut status = MMStatus::new(Status::Dnd, session.user_id.clone());
            status.send(session);
            self.used = true;
        } else if !watched_app_found && self.used {
            let mut status = MMStatus::new(Status::Online, session.user_id.clone());
            status.send(session);
            self.used = false;
        }
        Ok(self)
    }
}
