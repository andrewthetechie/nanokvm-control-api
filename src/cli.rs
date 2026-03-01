use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "nanokvm-control-api",
    version,
    about = "NanoKVM Redfish BMC Emulator"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the Redfish API server
    Serve {
        /// Path to config file
        #[arg(short, long, default_value = "/etc/nanokvm/config.toml")]
        config: String,
    },
    /// Run ISO cleanup once and exit
    Cleanup {
        /// Path to config file
        #[arg(short, long, default_value = "/etc/nanokvm/config.toml")]
        config: String,
        /// Show what would be cleaned up without deleting
        #[arg(long)]
        dry_run: bool,
    },
}
