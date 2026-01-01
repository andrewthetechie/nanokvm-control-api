use flexi_logger::{Duplicate, Logger};
use log::LevelFilter;
use std::env;
use std::error::Error;
use std::path::Path;
use tiny_http::{Header, Method, Response, Server, StatusCode};
mod config;
mod control;
use crate::config::{Config, read_config};
use crate::control::{StateManager, handle_input, handle_power};
use std::sync::Arc;

// Helper to extract action from query parameters
fn extract_action(query_part: Option<&str>) -> Option<&str> {
    query_part.and_then(|q| {
        q.split('&').find_map(|param| {
            let mut kv = param.split('=');
            if kv.next() == Some("action") {
                kv.next()
            } else {
                None
            }
        })
    })
}

fn init_logger(config: &Config) -> Result<(), Box<dyn Error>> {
    let log_level = match config.log_level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    let mut logger = Logger::try_with_str(format!("{}", log_level))?;

    if config.log_file.to_lowercase() == "stdout" {
        logger = logger.log_to_stdout().duplicate_to_stderr(Duplicate::Error);
    } else {
        let log_path = Path::new(&config.log_file);
        let directory = log_path.parent().unwrap_or_else(|| Path::new("."));
        let filename = log_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("log");

        logger = logger.log_to_file(
            flexi_logger::FileSpec::default()
                .directory(directory)
                .basename(filename),
        );
    }

    logger.start()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let config: Config = read_config();
    init_logger(&config)?;
    log::info!("Loaded config: {:?}", config);

    log::info!("Initializing system state...");
    let state_manager = Arc::new(StateManager::new(config.state_storage_path.clone())?);
    log::info!("State manager initialized");

    let server_url = format!("{}:{}", config.server_host, config.server_port);
    let server = Server::http(server_url.clone()).unwrap();
    log::info!("Control API running on {}", server_url);

    for request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        let state_manager = Arc::clone(&state_manager);

        log::debug!("received request -> method: {:?}, url: {:?}", method, url);

        // Split URL into path and query
        let (path_part, query_part) = match url.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (&url[..], None),
        };

        let parts = path_part
            .trim_start_matches('/')
            .split('/')
            .collect::<Vec<_>>();

        let response = match (method, parts.as_slice()) {
            // GET /
            (Method::Get, [""]) => {
                let v = env!("CARGO_PKG_VERSION");
                Response::from_string(format!("Hello from Control API {}", v))
            }

            // GET /health
            (Method::Get, ["health"]) => Response::from_string("OK"),

            // GET /status
            (Method::Get, ["status"]) => {
                let json = r#"{
                    "current_input": 2,
                    "power": {
                        "1": "on",
                        "2": "off",
                        "3": "on",
                        "4": "off"
                    }
                }"#;

                let mut resp = Response::from_string(json);
                resp.add_header("Content-Type: application/json".parse::<Header>().unwrap());
                resp
            }

            // POST/PUT /input/{id}
            (Method::Post, ["input", id]) | (Method::Put, ["input", id]) => {
                handle_input(&state_manager, id)
            }

            // POST/PUT /power/soft/{id}
            (Method::Post, ["power", "soft", id]) | (Method::Put, ["power", "soft", id]) => {
                match extract_action(query_part) {
                    Some(action) => handle_power(&state_manager, "soft", id, action),
                    None => Response::from_string("Missing required 'action' query parameter")
                        .with_status_code(StatusCode(400)),
                }
            }

            // POST/PUT /power/hard/{id}
            (Method::Post, ["power", "hard", id]) | (Method::Put, ["power", "hard", id]) => {
                match extract_action(query_part) {
                    Some(action) => handle_power(&state_manager, "hard", id, action),
                    None => Response::from_string("Missing required 'action' query parameter")
                        .with_status_code(StatusCode(400)),
                }
            }

            _ => Response::from_string("Not Found").with_status_code(StatusCode(404)),
        };

        request.respond(response)?;
    }

    Ok(())
}
