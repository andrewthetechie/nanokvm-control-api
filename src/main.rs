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

            let app_config = match config::load_config(&config).await {
                Ok(c) => std::sync::Arc::new(c),
                Err(e) => {
                    tracing::error!("Failed to load config: {}", e);
                    std::process::exit(1);
                }
            };

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
            let nanokvm_client: std::sync::Arc<dyn nanokvm::NanoKvmClient> =
                std::sync::Arc::new(nanokvm::client::HttpNanoKvmClient::new(&app_config.nanokvm));
            let virtual_media = virtual_media::manager::VirtualMediaManager::new(
                &app_config.virtual_media,
                nanokvm_client,
            );

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
