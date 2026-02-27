#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use ::lib::config::Args;
use ::lib::*;
use anyhow::{Context, Result};

#[paw::main]
fn main(args: Args) -> Result<()> {
    if args.expose_secrets {
        ::lib::secret::enable_expose();
    }
    let args = args
        .merge_config_and_params()?
        // Retrieve token if possible
        .update_secret_with_command()
        .context("Get secret from mm_secret_cmd")?
        .update_secret_with_keyring()
        .context("Get secret from OS keyring")?;
    setup_tracing(&args).context("Setting up tracing")?;
    let config = args.validate().context("Validating configuration")?;
    let status_dict = prepare_status(&config).context("Building custom status messages")?;
    get_wifi_and_update_status_loop(config, status_dict)?;
    Ok(())
}
