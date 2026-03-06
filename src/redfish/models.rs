use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceRoot {
    #[serde(rename = "@odata.type")]
    pub odata_type: &'static str,
    #[serde(rename = "@odata.id")]
    pub odata_id: &'static str,
    pub id: &'static str,
    pub name: &'static str,
    pub systems: ResourceLink,
    pub managers: ResourceLink,
    pub task_service: ResourceLink,
    pub update_service: ResourceLink, // Even if stubbed
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResourceLink {
    #[serde(rename = "@odata.id")]
    pub odata_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Collection {
    #[serde(rename = "@odata.type")]
    pub odata_type: &'static str,
    #[serde(rename = "@odata.id")]
    pub odata_id: String,
    pub name: String,
    pub members: Vec<ResourceLink>,
    #[serde(rename = "Members@odata.count")]
    pub members_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ComputerSystem {
    #[serde(rename = "@odata.type")]
    pub odata_type: &'static str,
    #[serde(rename = "@odata.id")]
    pub odata_id: String,
    pub id: String,
    pub name: String,
    pub power_state: &'static str, // "On" or "Off"
    pub boot: BootSettings,
    pub actions: SystemActions,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BootSettings {
    pub boot_source_override_enabled: &'static str,
    pub boot_source_override_target: String,
    pub boot_source_override_mode: &'static str,
    #[serde(rename = "BootSourceOverrideTarget@Redfish.AllowableValues")]
    pub allowable_values: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemActions {
    #[serde(rename = "#ComputerSystem.Reset")]
    pub reset: ResetAction,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResetAction {
    pub target: String,
    #[serde(rename = "ResetType@Redfish.AllowableValues")]
    pub allowable_values: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Manager {
    #[serde(rename = "@odata.type")]
    pub odata_type: &'static str,
    #[serde(rename = "@odata.id")]
    pub odata_id: String,
    pub id: String,
    pub name: String,
    pub virtual_media: ResourceLink,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct VirtualMedia {
    #[serde(rename = "@odata.type")]
    pub odata_type: &'static str,
    #[serde(rename = "@odata.id")]
    pub odata_id: String,
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_name: String,
    pub inserted: bool,
    pub write_protected: bool,
    pub connected_via: &'static str,
    pub supported_media_types: Vec<&'static str>,
    pub transfer_method: &'static str,
    pub transfer_protocol_type: &'static str,
    pub actions: VirtualMediaActions,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct VirtualMediaActions {
    #[serde(rename = "#VirtualMedia.InsertMedia")]
    pub insert_media: ActionTarget,
    #[serde(rename = "#VirtualMedia.EjectMedia")]
    pub eject_media: ActionTarget,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionTarget {
    pub target: String,
}
