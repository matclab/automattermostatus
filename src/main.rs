#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
use anyhow::{Context, Result};

use ::lib::config::Args;
use ::lib::*;

#[paw::main]
fn main(args: Args) -> Result<()> {
    setup_tracing(&args).context("Setting up tracing")?;
    let args = merge_config_and_params(&args)?;
    // Compute token if needed
    let args = update_token_with_command(args).context("Get private token from mm_token_cmd")?;
    let status_dict = prepare_status(&args).context("Building custom status messages")?;
    get_wifi_and_update_status_loop(args, status_dict)?;
    Ok(())
}
