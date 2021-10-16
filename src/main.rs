use anyhow::Result;
use structopt::clap::AppSettings;



#[derive(structopt::StructOpt)]
/// Automate mattermost status with the help of wifi network
///
/// Use current available SSID of wifi networks to automate your mattermost status.
/// This program is mean to be call regularly and will update status according to the config file
#[structopt(global_settings(&[AppSettings::ColoredHelp, AppSettings::ColorAuto]))]
struct Args {
    /// wifi interface name
    #[structopt(short, long, env )]
    interface_name: String,

    /// work SSID substring
    ///
    /// string that shall be contains in a visible SSID to be considered at work
    #[structopt(short="W", long, env )]
    work_ssid: String,

    /// home SSID substring
    ///
    /// string that shall be contains in a visible SSID to be considered at home
    #[structopt(short="H", long, env )]
    home_ssid: String,

    /// mattermost URL
    #[structopt(short="u", long, env )]
    mm_url: String,

    /// mattermost private Token
    #[structopt(long, env, hide_env_values = true)]
    mm_token: Option<String>,

    /// mattermost private Token command
    #[structopt(long, env )]
    mm_token_cmd: Option<String>,

    /// directory for state file
    #[structopt(long, default_value = "~/.cache/automattermostatus.state", env)]
    state_file: String,


    /// A level of verbosity, and can be used multiple times
    #[structopt(short, long, parse(from_occurrences))]
    verbose: i32,
}

#[paw::main]
fn main(args: Args) -> Result<(), std::io::Error> {
    // Gets a value for config if supplied by user, or defaults to "default.conf"
    println!("Using input file: {}", args.interface_name);

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match args.verbose {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        _ => println!("Don't be ridiculous"),
    }
    Ok(())

}
