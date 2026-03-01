use crate::auth::RequireAuth;
use crate::state::{PowerState, StateManager};
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<crate::state::AppState> {
    Router::new().route("/v1/power-state", get(get_power_state).put(set_power_state))
}

#[derive(Serialize, Deserialize)]
pub struct PowerStateDto {
    pub state: String,
}

async fn get_power_state(
    State(state_manager): State<StateManager>,
    _auth: RequireAuth,
) -> Json<PowerStateDto> {
    let state_str = match state_manager.get_power_state().await {
        PowerState::On => "On",
        PowerState::Off => "Off",
        PowerState::Unknown => "Unknown",
    };
    Json(PowerStateDto {
        state: state_str.to_string(),
    })
}

async fn set_power_state(
    State(state_manager): State<StateManager>,
    _auth: RequireAuth,
    Json(payload): Json<PowerStateDto>,
) -> Json<PowerStateDto> {
    let new_state = match payload.state.as_str() {
        "On" => PowerState::On,
        "Off" => PowerState::Off,
        _ => PowerState::Unknown,
    };
    state_manager.set_power_state(new_state).await;

    let state_str = match state_manager.get_power_state().await {
        PowerState::On => "On",
        PowerState::Off => "Off",
        PowerState::Unknown => "Unknown",
    };
    Json(PowerStateDto {
        state: state_str.to_string(),
    })
}
