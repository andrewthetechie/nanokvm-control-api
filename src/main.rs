pub mod auth;
pub mod cli;
pub mod config;
pub mod error;
pub mod nanokvm;
pub mod power;
pub mod redfish;
pub mod state;
pub mod virtual_media;

use clap::Parser;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Serve { config } => {
            tracing::info!("Starting NanoKVM Control API (Redfish rebuild)");
            tracing::debug!("Config path: {}", config);

            // Build the router (empty for now)
            let app = axum::Router::new().merge(redfish::routes());

            let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
            axum::serve(listener, app).await?;
        }
        cli::Commands::Cleanup { config, dry_run } => {
            tracing::info!("Running ISO cleanup (dry_run: {})", dry_run);
            tracing::debug!("Config path: {}", config);
            // Cleanup logic will go here
        }
    }

    Ok(())
}
