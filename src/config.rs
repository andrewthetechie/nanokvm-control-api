use std::env;


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
    pub i2c_bus: String,
    pub i2c_address: String
}


pub fn read_config() -> Config {
    Config {
        server_port: get_env_u16("SERVER_PORT", 8000),
        server_host: get_env_string("SERVER_HOST", "0.0.0.0"),
        button_press_delay_ms: get_env_float("BUTTON_PRESS_DELAY_MS", 30.0),
        soft_power_short_press_ms: get_env_float("SOFT_POWER_SHORT_PRESS_MS", 30.0),
        soft_power_long_press_ms: get_env_float("SOFT_POWER_LONG_PRESS_MS", 90.0),
        hard_power_delay_ms: get_env_float("HARD_POWER_DELAY_MS", 30.0),
        power_default_state: get_env_u8("POWER_DEFAULT_STATE", 0),
        state_storage_path: get_env_string("STATE_STORAGE_PATH", "/etc/control_apl/state.json"),
        log_level: get_env_string("LOG_LEVEL", "info"),
        log_file: get_env_string("LOG_FILE", "stdout"),
        i2c_bus: get_env_string("I2C_BUS", "/dev/i2c-1"),
        i2c_address: get_env_string("I2C_ADDRESS", "0x20"),
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
