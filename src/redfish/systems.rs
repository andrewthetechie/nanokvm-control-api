use crate::auth::RequireAuth;
use crate::redfish::models::*;
use crate::state::{PowerState, StateManager};
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;

pub fn routes() -> Router<crate::state::AppState> {
    Router::new()
        .route("/", get(list_systems))
        .route("/1", get(get_system).patch(patch_system))
        .route("/1/Actions/ComputerSystem.Reset", post(reset_system))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResetRequest {
    pub reset_type: String, // "On", "ForceOff", "ForceRestart", "GracefulShutdown"
}

async fn reset_system(
    State(power_controller): State<Arc<dyn crate::power::PowerController>>,
    State(state_manager): State<StateManager>,
    _auth: RequireAuth,
    Json(payload): Json<ResetRequest>,
) -> StatusCode {
    let action_res: Result<(), crate::error::AppError> = match payload.reset_type.as_str() {
        "On" | "GracefulShutdown" | "GracefulRestart" => {
            power_controller.press_power_button().await
        }
        "ForceOff" => power_controller.force_off().await,
        "ForceRestart" => {
            let _ = power_controller.force_off().await;
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            power_controller.press_power_button().await
        }
        _ => return StatusCode::BAD_REQUEST,
    };

    if action_res.is_ok() {
        // Optimistically update the state
        match payload.reset_type.as_str() {
            "On" | "ForceRestart" | "GracefulRestart" => {
                state_manager.set_power_state(PowerState::On).await
            }
            "ForceOff" | "GracefulShutdown" => state_manager.set_power_state(PowerState::Off).await,
            _ => {}
        }
        StatusCode::NO_CONTENT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PatchSystemRequest {
    pub boot: Option<PatchBoot>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PatchBoot {
    pub boot_source_override_target: Option<String>,
    #[allow(dead_code)]
    pub boot_source_override_enabled: Option<String>,
}

async fn patch_system(
    State(virtual_media): State<crate::virtual_media::manager::VirtualMediaManager>,
    _auth: RequireAuth,
    Json(payload): Json<PatchSystemRequest>,
) -> StatusCode {
    if let Some(boot) = payload.boot
        && let Some(target) = boot.boot_source_override_target
    {
        let res = match target.as_str() {
            "Pxe" => virtual_media.set_pxe_boot().await,
            _ => virtual_media.set_boot_from_disk().await,
        };
        if res.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    StatusCode::OK
}

async fn list_systems(_auth: RequireAuth) -> Json<Collection> {
    Json(Collection {
        odata_type: "#ComputerSystemCollection.ComputerSystemCollection",
        odata_id: "/redfish/v1/Systems".to_string(),
        name: "Computer System Collection".to_string(),
        members: vec![ResourceLink {
            odata_id: "/redfish/v1/Systems/1".to_string(),
        }],
        members_count: 1,
    })
}

async fn get_system(
    State(state_manager): State<StateManager>,
    _auth: RequireAuth,
) -> Json<ComputerSystem> {
    let power_state = match state_manager.get_power_state().await {
        PowerState::On => "On",
        PowerState::Off => "Off",
        PowerState::Unknown => "Unknown", // Redfish expects On/Off/PoweringOn/PoweringOff, but we'll return Unknown if we don't know
    };

    Json(ComputerSystem {
        odata_type: "#ComputerSystem.v1_20_0.ComputerSystem",
        odata_id: "/redfish/v1/Systems/1".to_string(),
        id: "1".to_string(),
        name: "NanoKVM Server".to_string(),
        power_state,
        boot: BootSettings {
            boot_source_override_enabled: "Once",
            boot_source_override_target: "Cd",
            boot_source_override_mode: "UEFI",
            allowable_values: vec!["None", "Pxe", "Cd", "Hdd"],
        },
        actions: SystemActions {
            reset: ResetAction {
                target: "/redfish/v1/Systems/1/Actions/ComputerSystem.Reset".to_string(),
                allowable_values: vec!["On", "ForceOff", "ForceRestart", "GracefulShutdown"],
            },
        },
    })
}
