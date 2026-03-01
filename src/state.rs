use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    Unknown,
    On,
    Off,
}

#[derive(Clone)]
pub struct StateManager {
    power_state: Arc<RwLock<PowerState>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            power_state: Arc::new(RwLock::new(PowerState::Unknown)),
        }
    }

    pub async fn get_power_state(&self) -> PowerState {
        *self.power_state.read().await
    }

    pub async fn set_power_state(&self, state: PowerState) {
        let mut w = self.power_state.write().await;
        *w = state;
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests remain the same...
    use super::*;

    #[tokio::test]
    async fn test_state_manager_initial_state() {
        let manager = StateManager::new();
        assert_eq!(manager.get_power_state().await, PowerState::Unknown);
    }

    #[tokio::test]
    async fn test_state_manager_set_state() {
        let manager = StateManager::new();
        manager.set_power_state(PowerState::On).await;
        assert_eq!(manager.get_power_state().await, PowerState::On);

        manager.set_power_state(PowerState::Off).await;
        assert_eq!(manager.get_power_state().await, PowerState::Off);
    }
}

// Ensure the AppState uses Clone and FromRef for axum extractors
use crate::config::AppConfig;
use crate::power::PowerController;
use crate::virtual_media::manager::VirtualMediaManager;
use axum::extract::FromRef;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub state_manager: StateManager,
    pub power_controller: Arc<dyn PowerController>,
    pub virtual_media: VirtualMediaManager,
}

impl FromRef<AppState> for Arc<AppConfig> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for StateManager {
    fn from_ref(state: &AppState) -> Self {
        state.state_manager.clone()
    }
}

impl FromRef<AppState> for Arc<dyn PowerController> {
    fn from_ref(state: &AppState) -> Self {
        state.power_controller.clone()
    }
}

impl FromRef<AppState> for VirtualMediaManager {
    fn from_ref(state: &AppState) -> Self {
        state.virtual_media.clone()
    }
}
