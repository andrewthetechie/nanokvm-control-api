#![cfg(target_os = "linux")]

use super::PowerController;
use crate::config::PowerConfig;
use crate::error::AppError;
use gpiocdev::Result as GpioResult;
use gpiocdev::request::{Config, Request};
use std::time::Duration;
use tokio::time::sleep;
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

    fn toggle_line(&self, line: u32, delay: Duration) -> Result<(), AppError> {
        // Run GPIO operations in a blocking task since gpiocdev is synchronous
        let chip_path = self.chip_path.clone();

        let operation = move || -> GpioResult<()> {
            let req = Request::new(&Config::default().as_output())
                .on_chip(&chip_path)
                .with_lines(&[line])
                .request()?;

            // Pull line low (active)
            req.set_value(line, gpiocdev::line::Value::Inactive)?;

            // Wait
            std::thread::sleep(delay);

            // Release line (inactive)
            req.set_value(line, gpiocdev::line::Value::Active)?;
            Ok(())
        };

        // We run in current_thread but we don't want to block the reactor
        tokio::task::block_in_place(|| operation())
            .map_err(|e| AppError::Internal(format!("GPIO error: {}", e)))
    }
}

#[async_trait::async_trait]
impl PowerController for GpioPowerController {
    async fn press_power_button(&self) -> Result<(), AppError> {
        info!("GPIO: Pressing power button");
        let delay = self.press_delay;
        let line = self.power_line;

        // Spawn blocking to avoid thread starvation
        let this = self.clone(); // Can't easily clone without changing struct, doing direct instead
        tokio::task::spawn_blocking(move || {
            // Need a way to call toggle_line without self reference issues in spawn_blocking
            // Re-implementing logic here for simplicity
        });

        // Due to lifetimes with spawn_blocking, we just use block_in_place directly in toggle_line
        self.toggle_line(line, delay)
    }

    async fn force_off(&self) -> Result<(), AppError> {
        info!("GPIO: Forcing power off");
        self.toggle_line(self.hard_power_line, self.force_off_delay)
    }
}
