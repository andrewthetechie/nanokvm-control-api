use serde::Deserialize;
use std::env;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub power: PowerConfig,
    pub nanokvm: NanoKvmConfig,
    pub virtual_media: VirtualMediaConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub enabled: bool,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PowerConfig {
    pub enable_gpio: bool,
    pub gpio_chip: String,
    pub power_button_line: u32,
    pub hard_power_line: u32,
    pub button_press_delay_ms: u64,
    pub force_off_delay_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NanoKvmConfig {
    #[serde(default)]
    pub use_mock: bool,
    pub base_url: String,
    pub auth_token: Option<String>,
}

impl NanoKvmConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.use_mock && self.auth_token.is_none() {
            return Err("nanokvm.auth_token is required when use_mock is false".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct VirtualMediaConfig {
    pub isos_dir: String,
    pub boot_from_disk_iso: String,
    pub pxe_boot_iso: String,
    pub download_timeout_secs: u64,
    pub cleanup_ttl_secs: u64,
}

pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path).await?;
    let mut config: AppConfig = toml::from_str(&contents)?;

    // Env var overrides
    if let Ok(port) = env::var("NANOKVM_SERVER_PORT") {
        config.server.port = port.parse()?;
    }
    if let Ok(host) = env::var("NANOKVM_SERVER_HOST") {
        config.server.host = host;
    }

    // Auth overrides
    if let Ok(enabled) = env::var("NANOKVM_AUTH_ENABLED") {
        config.auth.enabled = enabled.parse()?;
    }
    if let Ok(user) = env::var("NANOKVM_AUTH_USERNAME") {
        config.auth.username = Some(user);
    }
    if let Ok(pass) = env::var("NANOKVM_AUTH_PASSWORD") {
        config.auth.password = Some(pass);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_load_config() {
        let toml_content = r#"
            [server]
            host = "127.0.0.1"
            port = 8000

            [auth]
            enabled = false

            [power]
            enable_gpio = false
            gpio_chip = "/dev/gpiochip1"
            power_button_line = 3
            hard_power_line = 4
            button_press_delay_ms = 500
            force_off_delay_ms = 5000

            [nanokvm]
            base_url = "http://localhost:8080"

            [virtual_media]
            isos_dir = "/data/isos"
            boot_from_disk_iso = "disk.iso"
            pxe_boot_iso = "pxe.iso"
            download_timeout_secs = 600
            cleanup_ttl_secs = 86400
        "#;

        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", toml_content).unwrap();

        let config = load_config(file.path()).await.unwrap();
        assert_eq!(config.server.port, 8000);
        assert!(!config.auth.enabled);
        assert_eq!(config.power.power_button_line, 3);
        assert_eq!(config.virtual_media.isos_dir, "/data/isos");

        // Test env override
        unsafe {
            env::set_var("NANOKVM_SERVER_PORT", "9000");
        }
        let config = load_config(file.path()).await.unwrap();
        assert_eq!(config.server.port, 9000);
        unsafe {
            env::remove_var("NANOKVM_SERVER_PORT");
        }
    }

    #[test]
    fn test_nanokvm_config_validate_mock_no_token() {
        let config = NanoKvmConfig {
            use_mock: true,
            base_url: "http://localhost:8080".to_string(),
            auth_token: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_nanokvm_config_validate_real_no_token() {
        let config = NanoKvmConfig {
            use_mock: false,
            base_url: "http://10.10.0.208".to_string(),
            auth_token: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_nanokvm_config_validate_real_with_token() {
        let config = NanoKvmConfig {
            use_mock: false,
            base_url: "http://10.10.0.208".to_string(),
            auth_token: Some("my-jwt-token".to_string()),
        };
        assert!(config.validate().is_ok());
    }
}
