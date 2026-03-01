//! Mock implementation
use super::PowerController;
use crate::error::AppError;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone, Default)]
pub struct MockPowerController {
    pressed: Arc<Mutex<bool>>,
    forced_off: Arc<Mutex<bool>>,
}

impl MockPowerController {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub async fn was_pressed(&self) -> bool {
        *self.pressed.lock().await
    }

    #[cfg(test)]
    pub async fn was_forced_off(&self) -> bool {
        *self.forced_off.lock().await
    }
}

#[async_trait::async_trait]
impl PowerController for MockPowerController {
    async fn press_power_button(&self) -> Result<(), AppError> {
        info!("MOCK: Power button pressed");
        *self.pressed.lock().await = true;
        Ok(())
    }

    async fn force_off(&self) -> Result<(), AppError> {
        info!("MOCK: Force off triggered");
        *self.forced_off.lock().await = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_press() {
        let mock = MockPowerController::new();
        mock.press_power_button().await.unwrap();
        assert!(mock.was_pressed().await);
        assert!(!mock.was_forced_off().await);
    }

    #[tokio::test]
    async fn test_mock_force_off() {
        let mock = MockPowerController::new();
        mock.force_off().await.unwrap();
        assert!(!mock.was_pressed().await);
        assert!(mock.was_forced_off().await);
    }
}
