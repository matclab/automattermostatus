#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use ::lib::config::Args;
use ::lib::*;
use anyhow::{Context, Result};

#[paw::main]
fn main(args: Args) -> Result<()> {
    setup_tracing(&args).context("Setting up tracing")?;
    let args = merge_config_and_params(&args)?
        // Retrieve token if possible
        .update_token_with_command()
        .context("Get private token from mm_token_cmd")?
        .update_token_with_keyring()
        .context("Get provate token from OS keyring")?;
    let status_dict = prepare_status(&args).context("Building custom status messages")?;
    get_wifi_and_update_status_loop(args, status_dict)?;
    Ok(())
}
