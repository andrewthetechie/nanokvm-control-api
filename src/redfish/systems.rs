use crate::auth::RequireAuth;
use crate::redfish::models::*;
use crate::state::{PowerState, StateManager};
use axum::{Json, Router, extract::State, routing::get};

pub fn routes() -> Router<crate::state::AppState> {
    Router::new()
        .route("/", get(list_systems))
        .route("/1", get(get_system))
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
