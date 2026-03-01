#![cfg(target_os = "linux")]

use super::PowerController;
use crate::config::PowerConfig;
use crate::error::AppError;
use gpiocdev::request::{Builder, Config};

use gpiocdev::Result as GpioResult;
use std::time::Duration;
use tracing::info;

pub struct GpioPowerController {
    chip_path: String,
    power_line: u32,
    hard_power_line: u32,
    press_delay: Duration,
    force_off_delay: Duration,
}

impl GpioPowerController {
    pub fn new(config: &PowerConfig) -> Self {
        Self {
            chip_path: config.gpio_chip.clone(),
            power_line: config.power_button_line,
            hard_power_line: config.hard_power_line,
            press_delay: Duration::from_millis(config.button_press_delay_ms),
            force_off_delay: Duration::from_millis(config.force_off_delay_ms),
        }
    }
    async fn toggle_line(&self, line: u32, delay: Duration) -> Result<(), AppError> {
        // Run GPIO operations in a blocking task since gpiocdev is synchronous
        let chip_path = self.chip_path.clone();

        let operation = move || -> GpioResult<()> {
            let mut config = Config::default();
            config.as_output(gpiocdev::line::Value::Inactive);

            let mut builder = Builder::default();
            builder.on_chip(&chip_path);
            builder.with_lines(&[line]);
            builder.with_config(config);
            let req = builder.request()?;

            // Pull line low (active)
            req.set_value(line, gpiocdev::line::Value::Inactive)?;

            // Wait
            std::thread::sleep(delay);

            // Release line (inactive)
            req.set_value(line, gpiocdev::line::Value::Active)?;
            Ok(())
        };

        tokio::task::spawn_blocking(operation)
            .await
            .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?
            .map_err(|e| AppError::Internal(format!("GPIO error: {}", e)))
    }
}

#[async_trait::async_trait]
impl PowerController for GpioPowerController {
    async fn press_power_button(&self) -> Result<(), AppError> {
        info!("GPIO: Pressing power button");
        let delay = self.press_delay;
        let line = self.power_line;

        // Due to lifetimes with spawn_blocking, we just use block_in_place directly in toggle_line
        self.toggle_line(line, delay).await
    }

    async fn force_off(&self) -> Result<(), AppError> {
        info!("GPIO: Forcing power off");
        self.toggle_line(self.hard_power_line, self.force_off_delay)
            .await
    }
}
