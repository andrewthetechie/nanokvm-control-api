pub mod auth;
mod cli;
mod config;
mod error;
mod management;
mod nanokvm;
mod power;
mod redfish;
mod state;
mod virtual_media;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config, .. } => {
            tracing::info!("Starting NanoKVM Control API (Redfish rebuild)");
            tracing::debug!("Config path: {}", config);

            let app_config = match config::load_config(&config).await {
                Ok(c) => std::sync::Arc::new(c),
                Err(e) => {
                    tracing::error!("Failed to load config: {}", e);
                    std::process::exit(1);
                }
            };

            // Validate NanoKVM config (auth_token required when not using mock)
            if let Err(e) = app_config.nanokvm.validate() {
                tracing::error!("Config validation failed: {}", e);
                std::process::exit(1);
            }

            // Initialize Power Controller
            #[cfg(target_os = "linux")]
            let power_controller: std::sync::Arc<dyn power::PowerController> =
                if app_config.power.enable_gpio {
                    std::sync::Arc::new(power::gpio::GpioPowerController::new(&app_config.power))
                } else {
                    std::sync::Arc::new(power::mock::MockPowerController::new())
                };

            #[cfg(not(target_os = "linux"))]
            let power_controller: std::sync::Arc<dyn power::PowerController> =
                std::sync::Arc::new(power::mock::MockPowerController::new());

            // Initialize Virtual Media Manager
            let nanokvm_client: std::sync::Arc<dyn nanokvm::NanoKvmClient> = if app_config
                .nanokvm
                .use_mock
            {
                std::sync::Arc::new(nanokvm::mock::MockNanoKvmClient::new())
            } else {
                std::sync::Arc::new(nanokvm::client::HttpNanoKvmClient::new(&app_config.nanokvm))
            };
            let virtual_media = virtual_media::manager::VirtualMediaManager::new(
                &app_config.virtual_media,
                nanokvm_client,
            );

            // Default to mounting disk boot ISO on startup
            if let Err(e) = virtual_media.set_boot_from_disk().await {
                tracing::warn!("Failed to mount initial disk-boot ISO: {}", e);
            }

            // Create App State
            let state = state::AppState {
                config: app_config.clone(),
                state_manager: state::StateManager::new(),
                power_controller,
                virtual_media,
            };

            // Setup Router
            let app = axum::Router::new()
                .nest("/redfish", redfish::routes())
                .nest("/api", management::routes())
                .with_state(state);

            let addr = format!("{}:{}", app_config.server.host, app_config.server.port);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            tracing::info!("listening on {}", listener.local_addr().unwrap());
            axum::serve(listener, app).await.unwrap();
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
