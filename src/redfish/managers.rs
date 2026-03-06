use crate::auth::RequireAuth;
use crate::redfish::models::*;
use crate::state::AppState;
use crate::virtual_media::manager::VirtualMediaManager;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_managers))
        .route("/1", get(get_manager))
        .route("/1/VirtualMedia", get(list_virtual_media))
        .route("/1/VirtualMedia/Cd", get(get_virtual_media_cd))
        .route(
            "/1/VirtualMedia/Cd/Actions/VirtualMedia.InsertMedia",
            post(insert_media),
        )
        .route(
            "/1/VirtualMedia/Cd/Actions/VirtualMedia.EjectMedia",
            post(eject_media),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InsertMediaRequest {
    pub image: String,
}

async fn insert_media(
    State(virtual_media): State<VirtualMediaManager>,
    _auth: RequireAuth,
    Json(payload): Json<InsertMediaRequest>,
) -> StatusCode {
    match virtual_media.insert_media(&payload.image).await {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(e) => {
            tracing::error!("InsertMedia failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn eject_media(
    State(virtual_media): State<VirtualMediaManager>,
    _auth: RequireAuth,
) -> StatusCode {
    let client = virtual_media.client();
    if client.unmount_iso().await.is_ok() {
        virtual_media.clear_mounted_iso().await;
        StatusCode::NO_CONTENT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn list_managers(_auth: RequireAuth) -> Json<Collection> {
    Json(Collection {
        odata_type: "#ManagerCollection.ManagerCollection",
        odata_id: "/redfish/v1/Managers".to_string(),
        name: "Manager Collection".to_string(),
        members: vec![ResourceLink {
            odata_id: "/redfish/v1/Managers/1".to_string(),
        }],
        members_count: 1,
    })
}

async fn get_manager(_auth: RequireAuth) -> Json<Manager> {
    Json(Manager {
        odata_type: "#Manager.v1_14_0.Manager",
        odata_id: "/redfish/v1/Managers/1".to_string(),
        id: "1".to_string(),
        name: "NanoKVM BMC".to_string(),
        virtual_media: ResourceLink {
            odata_id: "/redfish/v1/Managers/1/VirtualMedia".to_string(),
        },
    })
}

async fn list_virtual_media(_auth: RequireAuth) -> Json<Collection> {
    Json(Collection {
        odata_type: "#VirtualMediaCollection.VirtualMediaCollection",
        odata_id: "/redfish/v1/Managers/1/VirtualMedia".to_string(),
        name: "Virtual Media Collection".to_string(),
        members: vec![ResourceLink {
            odata_id: "/redfish/v1/Managers/1/VirtualMedia/Cd".to_string(),
        }],
        members_count: 1,
    })
}

async fn get_virtual_media_cd(
    State(virtual_media): State<VirtualMediaManager>,
    _auth: RequireAuth,
) -> Json<VirtualMedia> {
    let mounted_iso = virtual_media.get_mounted_iso().await;
    let inserted = mounted_iso.is_some();
    let image = mounted_iso.clone().unwrap_or_default();

    Json(VirtualMedia {
        odata_type: "#VirtualMedia.v1_5_0.VirtualMedia",
        odata_id: "/redfish/v1/Managers/1/VirtualMedia/Cd".to_string(),
        id: "Cd".to_string(),
        name: "Virtual CD/DVD".to_string(),
        image: image.clone(),
        image_name: image,
        inserted,
        write_protected: true,
        connected_via: "URI",
        supported_media_types: vec!["CD", "DVD"],
        transfer_method: "Stream",
        transfer_protocol_type: "HTTPS",
        actions: VirtualMediaActions {
            insert_media: ActionTarget {
                target: "/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.InsertMedia"
                    .to_string(),
            },
            eject_media: ActionTarget {
                target: "/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.EjectMedia"
                    .to_string(),
            },
        },
    })
}
