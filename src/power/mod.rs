//! Power module
use crate::error::AppError;

pub mod gpio;
pub mod mock;

#[async_trait::async_trait]
pub trait PowerController: Send + Sync {
    /// Press the power button (short press).
    async fn press_power_button(&self) -> Result<(), AppError>;

    /// Hold the power button for a hard reset/force off.
    async fn force_off(&self) -> Result<(), AppError>;
}
