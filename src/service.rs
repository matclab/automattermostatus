//! Windows service integration.
//!
//! This module is only compiled on Windows (`#[cfg(target_os = "windows")]`
//! is applied in `lib.rs`).
//!
//! It provides:
//! - [`run_as_service`] — entry point called when the SCM starts the binary
//! - [`install_service`] / [`uninstall_service`] — register/remove the service
//! - [`setup_service_tracing`] — file-based tracing for service mode

use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info};
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};

use crate::config::Args;
use crate::shutdown::ShutdownSignal;
use crate::{get_wifi_and_update_status_loop, prepare_status};

/// Name used to register the service in the SCM.
const SERVICE_NAME: &str = "automattermostatus";

/// Display name shown in `services.msc`.
const SERVICE_DISPLAY_NAME: &str = "Automattermostatus";

/// Description shown in `services.msc`.
const SERVICE_DESCRIPTION: &str =
    "Automate your Mattermost custom status based on visible wifi SSIDs";

/// Directory where service logs are written.
const LOG_DIR: &str = r"C:\ProgramData\automattermostatus";

// Generate the FFI trampoline expected by `service_dispatcher::start`.
define_windows_service!(ffi_service_main, service_main);

/// Start the service dispatcher.
///
/// This is called from `main()` when the binary is invoked with `service run`.
/// The SCM calls back into [`service_main`] on a separate thread.
pub fn run_as_service() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("Failed to start service dispatcher")
}

/// Service entry point invoked by the SCM.
///
/// Runs on a thread spawned by the service dispatcher.
fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        error!("Service failed: {:#}", e);
    }
}

/// Inner service logic separated for `?` ergonomics.
fn run_service() -> Result<()> {
    let shutdown = ShutdownSignal::new();
    let shutdown_clone = shutdown.clone();

    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop => {
                shutdown_clone.request_shutdown();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })
        .context("Registering service control handler")?;

    // Report Running to the SCM.
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::ZERO,
            process_id: None,
        })
        .context("Reporting Running state")?;

    // Set up file-based tracing; keep the guard alive until the service stops.
    let _guard = setup_service_tracing();

    // Load configuration from file only (no CLI parsing in service mode).
    let args = Args::default()
        .merge_config_and_params()
        .context("Loading service configuration")?
        .update_secret_with_command()
        .context("Get secret from mm_secret_cmd")?
        .update_secret_with_keyring()
        .context("Get secret from OS keyring")?;
    let config = args
        .validate()
        .context("Validating service configuration")?;
    let status_dict = prepare_status(&config).context("Building custom status messages")?;

    if let Err(e) = get_wifi_and_update_status_loop(config, status_dict, shutdown) {
        error!("Main loop error: {:#}", e);
    }

    // Report Stopped to the SCM.
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::ZERO,
            process_id: None,
        })
        .context("Reporting Stopped state")?;

    Ok(())
}

/// Install the service in the Windows SCM.
pub fn install_service() -> Result<()> {
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
            .context("Opening service manager")?;

    let exe_path = std::env::current_exe().context("Getting current executable path")?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe_path,
        launch_arguments: vec![OsString::from("service"), OsString::from("run")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = manager
        .create_service(&service_info, ServiceAccess::CHANGE_CONFIG)
        .context("Creating service")?;
    service
        .set_description(SERVICE_DESCRIPTION)
        .context("Setting service description")?;

    info!("Service '{}' installed successfully", SERVICE_NAME);
    Ok(())
}

/// Uninstall the service from the Windows SCM.
///
/// If the service is running it will be stopped first.
pub fn uninstall_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("Opening service manager")?;

    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::STOP | ServiceAccess::QUERY_STATUS | ServiceAccess::DELETE,
        )
        .context("Opening service for uninstall")?;

    // Stop the service if it is running.
    if let Ok(status) = service.query_status() {
        if status.current_state != ServiceState::Stopped {
            info!("Stopping service before uninstall...");
            let _ = service.stop();
            // Wait a bit for the service to stop.
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    service.delete().context("Deleting service")?;
    info!("Service '{}' uninstalled successfully", SERVICE_NAME);
    Ok(())
}

/// Set up file-based tracing for service mode.
///
/// Logs are written to `C:\ProgramData\automattermostatus\` using daily
/// rotation. The returned [`tracing_appender::non_blocking::WorkerGuard`]
/// **must** be kept alive for the lifetime of the service — dropping it
/// flushes and stops the background writer.
fn setup_service_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = PathBuf::from(LOG_DIR);
    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(log_dir, "automattermostatus.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false).with_writer(non_blocking);
    let filter_layer = EnvFilter::try_new("info").expect("valid filter");

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    guard
}
