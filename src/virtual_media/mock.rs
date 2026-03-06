#![allow(dead_code)]
//! Mock implementation
use super::manager::VirtualMediaManager;
use crate::config::VirtualMediaConfig;
use std::sync::Arc;

pub fn create_mock_manager() -> (
    VirtualMediaManager,
    Arc<crate::virtual_media::mock_controller::MockMediaController>,
) {
    let config = VirtualMediaConfig {
        isos_dir: "/tmp/nanokvm_mock_isos".to_string(),
        boot_from_disk_iso: "disk.iso".to_string(),
        pxe_boot_iso: "pxe.iso".to_string(),
        download_timeout_secs: 600,
        cleanup_ttl_hours: 3600,
        configfs_lun_path: "/tmp/mock_configfs/lun.0".to_string(),
    };

    let media_controller =
        Arc::new(crate::virtual_media::mock_controller::MockMediaController::new());
    let manager = VirtualMediaManager::new(&config, media_controller.clone());

    (manager, media_controller)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_virtual_media_swapping() {
        let (manager, media_controller) = create_mock_manager();

        // Test PXE boot selection
        manager.set_pxe_boot().await.unwrap();
        let mounted = media_controller.get_mounted_iso().await.unwrap();
        assert!(mounted.ends_with("pxe.iso"));

        // Test Boot from disk selection
        manager.set_boot_from_disk().await.unwrap();
        let mounted = media_controller.get_mounted_iso().await.unwrap();
        assert!(mounted.ends_with("disk.iso"));
    }
}
