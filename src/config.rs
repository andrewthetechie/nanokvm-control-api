use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone)]
pub struct InputPinConfig {
    pub pin: u8,
    pub pushed_state: u8,
}

#[allow(dead_code)] // TODO: Remove this once we're using all of the config
#[derive(Debug)]
pub struct Config {
    pub server_port: u16,
    pub server_host: String,
    pub button_press_delay_ms: f32,
    pub soft_power_short_press_ms: f32,
    pub soft_power_long_press_ms: f32,
    pub hard_power_delay_ms: f32,
    pub power_default_state: u8,
    pub state_storage_path: String,
    pub log_level: String,
    pub log_file: String,
    pub usb_i2c_bus: String,
    pub usb_i2c_address: String,
    pub input_config: HashMap<u8, InputPinConfig>,
    pub power_soft_config: HashMap<u8, InputPinConfig>,
    pub power_hard_config: HashMap<u8, InputPinConfig>,
}

pub fn read_config() -> Config {
    Config {
        server_port: get_env_u16("SERVER_PORT", 8000),
        server_host: get_env_string("SERVER_HOST", "0.0.0.0"),
        button_press_delay_ms: get_env_float("BUTTON_PRESS_DELAY_MS", 50.0),
        soft_power_short_press_ms: get_env_float("SOFT_POWER_SHORT_PRESS_MS", 50.0),
        soft_power_long_press_ms: get_env_float("SOFT_POWER_LONG_PRESS_MS", 120.0),
        hard_power_delay_ms: get_env_float("HARD_POWER_DELAY_MS", 50.0),
        power_default_state: get_env_u8("POWER_DEFAULT_STATE", 0),
        state_storage_path: get_env_string("STATE_STORAGE_PATH", "./state.json"),
        log_level: get_env_string("LOG_LEVEL", "info"),
        log_file: get_env_string("LOG_FILE", "stdout"),
        usb_i2c_bus: get_env_string("USB_I2C_BUS", "/dev/i2c-5"),
        usb_i2c_address: get_env_string("USB_I2C_ADDRESS", "0x20"),
        input_config: get_env_input_config("INPUT_CONFIG", "1,0,0;2,1,0;3,2,0;4,3,0"),
        power_soft_config: get_env_input_config("POWER_SOFT_CONFIG", "1,4,0;2,5,0;3,6,0;4,7,0"),
        power_hard_config: get_env_input_config("POWER_HARD_CONFIG", "1,8,0;2,9,0;3,10,0;4,11,0"),
    }
}

fn get_env_float(key: &str, default: f32) -> f32 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_env_u8(key: &str, default: u8) -> u8 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_env_u16(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_env_string(key: &str, default: &str) -> String {
    env::var(key).unwrap_or(default.to_string())
}

fn get_env_input_config(key: &str, default: &str) -> HashMap<u8, InputPinConfig> {
    let config_str = env::var(key).unwrap_or(default.to_string());
    parse_input_config(&config_str)
}

fn parse_input_config(config_str: &str) -> HashMap<u8, InputPinConfig> {
    let mut map = HashMap::new();

    for entry in config_str.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let parts: Vec<&str> = entry.split(',').collect();
        if parts.len() != 3 {
            continue; // Skip malformed entries
        }

        let input_number = match parts[0].trim().parse::<u8>() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let pin_number = match parts[1].trim().parse::<u8>() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let pushed_state = match parts[2].trim().parse::<u8>() {
            Ok(n) if n == 0 || n == 1 => n,
            _ => 0, // Default to 0 for invalid pushed_state values
        };

        map.insert(
            input_number,
            InputPinConfig {
                pin: pin_number,
                pushed_state,
            },
        );
    }

    map
}
