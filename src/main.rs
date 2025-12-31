use tiny_http::{Server, Response, StatusCode, Method, Header};
use std::error::Error;
use std::env;
mod config;
mod control;
use crate::control::{handle_input, handle_power};
use crate::config::{read_config, Config};


fn main() -> Result<(), Box<dyn Error>> {
    let config: Config = read_config();
    println!("Loaded config: {:?}", config);

    println!("Initializing system state...");
    let _ = handle_input("1");

    let server_url = format!("{}:{}", config.server_host, config.server_port);
    let server = Server::http(server_url.clone()).unwrap();
    println!("Control API running on {}", server_url);

    for request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();

        println!(
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
