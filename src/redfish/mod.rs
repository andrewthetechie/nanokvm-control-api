pub mod actions;
pub mod models;
pub mod systems;

use crate::auth::RequireAuth;
use crate::state::AppState;
use axum::{Json, Router, routing::get};
use models::*;

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/", get(service_root))
}

async fn service_root(_auth: RequireAuth) -> Json<ServiceRoot> {
    Json(ServiceRoot {
        odata_type: "#ServiceRoot.v1_15_0.ServiceRoot",
        odata_id: "/redfish/v1/",
        id: "RootService",
        name: "NanoKVM Redfish Service",
        systems: ResourceLink {
            odata_id: "/redfish/v1/Systems".to_string(),
        },
        managers: ResourceLink {
            odata_id: "/redfish/v1/Managers".to_string(),
        },
        update_service: ResourceLink {
            odata_id: "/redfish/v1/UpdateService".to_string(),
        },
    })
}
