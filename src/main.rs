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

            // Load config first
            let app_config = match config::load_config(&config).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to load config: {}", e);
                    std::process::exit(1);
                }
            };

            if let Err(e) =
                virtual_media::cleanup::cleanup_old_isos(&app_config.virtual_media).await
            {
                tracing::error!("Cleanup task failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
