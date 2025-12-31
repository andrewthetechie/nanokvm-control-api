use tiny_http::{Server, Response, StatusCode, Method, Header};
use std::error::Error;
use std::env;

const VALID_IDS: [u8; 4] = [1, 2, 3, 4];

#[derive(Debug)]
struct Config {
    server_port: u16,
    server_host: String,
    button_press_delay_ms: f32,
    soft_power_short_press_ms: f32,
    soft_power_long_press_ms: f32,
    hard_power_delay_ms: f32,
    power_default_state: u8,
    state_storage_path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = read_config();
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

fn parse_id(id_str: &str) -> Result<u8, Response<std::io::Cursor<Vec<u8>>>> {
    if let Ok(id) = id_str.parse::<u8>() {
        if VALID_IDS.contains(&id) {
            return Ok(id);
        }
    }

    Err(Response::from_string("ID must be integer 1-4")
        .with_status_code(StatusCode(400)))
}

fn handle_input(id_str: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    match parse_id(id_str) {
        Ok(id) => {
            println!("Setting input to {}", id);
            Response::from_string(format!("Input {} selected", id))
        }
        Err(resp) => resp,
    }
}

fn handle_power(kind: &str, id_str: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    match parse_id(id_str) {
        Ok(id) => {
            println!("Power {} action triggered for {}", kind, id);
            Response::from_string(format!("Power {} action triggered for {}", kind, id))
        }
        Err(resp) => resp,
    }
}

fn read_config() -> Config {
    Config {
        server_port: get_env_u16("SERVER_PORT", 8000),
        server_host: get_env_string("SERVER_HOST", "0.0.0.0"),
        button_press_delay_ms: get_env_float("BUTTON_PRESS_DELAY_MS", 30.0),
        soft_power_short_press_ms: get_env_float("SOFT_POWER_SHORT_PRESS_MS", 30.0),
        soft_power_long_press_ms: get_env_float("SOFT_POWER_LONG_PRESS_MS", 90.0),
        hard_power_delay_ms: get_env_float("HARD_POWER_DELAY_MS", 30.0),
        power_default_state: get_env_u8("POWER_DEFAULT_STATE", 0),
        state_storage_path: env::var("STATE_STORAGE_PATH")
            .unwrap_or("/etc/control_apl/state.json".to_string()),
    }
}

fn get_env_float(key: &str, default: f32) -> f32 {
    env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn get_env_u8(key: &str, default: u8) -> u8 {
    env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn get_env_u16(key: &str, default: u16) -> u16 {
    env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn get_env_string(key: &str, default: &str) -> String {
    env::var(key).unwrap_or(default.to_string())
}
