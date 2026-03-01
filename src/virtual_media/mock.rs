#![allow(dead_code)]
//! Mock implementation
use super::manager::VirtualMediaManager;
use crate::config::VirtualMediaConfig;
use crate::nanokvm::mock::MockNanoKvmClient;
use std::sync::Arc;

pub fn create_mock_manager() -> (VirtualMediaManager, Arc<MockNanoKvmClient>) {
    let config = VirtualMediaConfig {
        isos_dir: "/tmp/nanokvm_mock_isos".to_string(),
        boot_from_disk_iso: "disk.iso".to_string(),
        pxe_boot_iso: "pxe.iso".to_string(),
        download_timeout_secs: 60,
        cleanup_ttl_secs: 3600,
    };

    let nanokvm = Arc::new(MockNanoKvmClient::new());
    let manager = VirtualMediaManager::new(&config, nanokvm.clone());

    (manager, nanokvm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_virtual_media_swapping() {
        let (manager, nanokvm) = create_mock_manager();

        // Test PXE boot selection
        manager.set_pxe_boot().await.unwrap();
        let mounted = nanokvm.get_mounted_iso().await.unwrap();
        assert!(mounted.ends_with("pxe.iso"));

        // Test Boot from disk selection
        manager.set_boot_from_disk().await.unwrap();
        let mounted = nanokvm.get_mounted_iso().await.unwrap();
        assert!(mounted.ends_with("disk.iso"));
    }
}
