use tiny_http::{Server, Response, StatusCode, Method, Header};
use std::error::Error;
use std::env;
use std::path::Path;
use log::LevelFilter;
use flexi_logger::{Logger, Duplicate};
mod config;
mod control;
use crate::control::{handle_input, handle_power};
use crate::config::{read_config, Config};


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
        logger = logger
            .log_to_stdout()
            .duplicate_to_stderr(Duplicate::Error);
    } else {
        let log_path = Path::new(&config.log_file);
        let directory = log_path.parent().unwrap_or_else(|| Path::new("."));
        let filename = log_path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        
        logger = logger.log_to_file(
            flexi_logger::FileSpec::default()
                .directory(directory)
                .basename(filename)
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
    let _ = handle_input("1");

    let server_url = format!("{}:{}", config.server_host, config.server_port);
    let server = Server::http(server_url.clone()).unwrap();
    log::info!("Control API running on {}", server_url);

    for request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();

        log::debug!(
            "received request -> method: {:?}, url: {:?}",
            method, url
        );

        let parts = url.trim_start_matches('/').split('/').collect::<Vec<_>>();

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
                resp.add_header(
                    "Content-Type: application/json"
                        .parse::<Header>()
                        .unwrap()
                );
                resp
            }

            // POST/PUT /input/{id}
            (Method::Post, ["input", id]) | (Method::Put, ["input", id]) => handle_input(id),

            // POST/PUT /power/soft/{id}
            (Method::Post, ["power", "soft", id])
            | (Method::Put, ["power", "soft", id]) => handle_power("soft", id),

            // POST/PUT /power/hard/{id}
            (Method::Post, ["power", "hard", id])
            | (Method::Put, ["power", "hard", id]) => handle_power("hard", id),

            _ => Response::from_string("Not Found").with_status_code(StatusCode(404)),
        };

        request.respond(response)?;
    }

    Ok(())
}
